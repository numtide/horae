// The Harvest connection, the CSV preview from the keyboard, runs whose report
// cannot be shown or stopped early, a failed status read, records with a new
// API id, and long identifiers on small screens. harvest-section.cjs covers
// page URLs, error groups, adding people, committing a preview and page fit.
// Run only against an isolated, seeded dev-login instance. Every importer
// request is intercepted, so no import, account change or Harvest call happens.
// HORAE_TEST_URL=http://127.0.0.1:8093 node crates/horae/tests/browser/harvest-importer-flows.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated fixture instance');
assert.ok(['127.0.0.1', 'localhost'].includes(new URL(base).hostname), 'Fixtures require a loopback instance');

const id = '019956ed-0000-7000-8000-000000000001';
const counts = () => ({ created: 0, updated: 0, skipped: 0, errored: 0 });
const report = mode => ({
  version: 2, source: 'csv', mode,
  summary: { clients: counts(), projects: counts(), tasks: counts(), people: counts(), time_entries: counts() },
  row_errors: [],
});
const job = (status = 'succeeded', mode = 'DryRun') => ({
  id, kind: 'harvest_csv_import', status, phase: status === 'running' ? 'time_entries' : null, error_kind: null,
  processed_count: 12, total_count: null, report: report(mode), last_error: null,
  created_at: '2026-09-15T10:30:00Z', finished_at: null, retry_availability: 'unavailable_state',
  mode, source: 'csv', commit_availability: mode === 'DryRun' && status === 'succeeded' ? 'available' : 'not_preview',
});
const connected = () => ({ configured: true, connected: true, account_id: 'fixture-account',
  token_expired: false, account_generation: 2, connection_revision: 4, has_provenance: false, active_imports: 0 });
const summaryOf = run => ({
  groups: run.report.row_errors.map(error => ({ entity: error.entity, kind: null, entry_kind: null, reason: error.reason, count: 1,
    samples: [{ location: error.source_location, reason: error.reason }] })),
  people: [], omitted_errors: 0, omitted_people: 0,
});
const longId = 'fixture'.repeat(40);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const failures = [];
  const errors = [];
  let connection, history, selected, statusFailure, summaryFailure, gate, starts, changes, disconnects, statusReads;
  page.on('pageerror', error => {
    // Leaving a page that still follows a running job aborts its status poll.
    if (error.name === 'AbortError') return;
    errors.push(error.message);
  });
  function reset() {
    connection = connected(); history = []; selected = job(); statusFailure = false; summaryFailure = false;
    gate = null; starts = 0; changes = 0; disconnects = 0; statusReads = [];
  }
  reset();
  // Only API calls: the section's own page URLs contain "harvest" too.
  await page.route(url => url.pathname.startsWith('/api/') && url.pathname.includes('harvest'), async route => {
    const request = route.request();
    const pathname = new URL(request.url()).pathname;
    const reply = value => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(value) });
    if (pathname === '/api/import/harvest/connection') return reply(connection);
    // Account changes reach the gate, then fail: the dialog shows the error and nothing changes.
    if (pathname.includes('harvest_change_account')) { changes++; if (gate) await gate; return route.abort('failed'); }
    if (pathname.includes('harvest_disconnect')) { disconnects++; if (gate) await gate; return route.abort('failed'); }
    if (pathname === '/api/import/harvest/history') return reply(history);
    if (pathname === '/api/import/harvest/status') {
      statusReads.push(request.postDataJSON().job_id);
      if (statusFailure) return route.abort('failed');
      return reply(selected);
    }
    if (pathname === '/api/import/harvest/error-summary') {
      if (summaryFailure) return route.abort('failed');
      return reply(summaryOf(selected));
    }
    if (pathname.startsWith('/api/import/harvest/csv-job/') || pathname === '/api/import/harvest/start') {
      starts++;
      return reply({ ...selected, status: 'queued', report: null });
    }
    errors.push(`Unrecognized importer request was blocked: ${pathname}`);
    return route.abort('failed');
  });
  async function check(name, body) {
    try { reset(); await body(); console.log(`PASS: ${name}`); }
    catch (error) { failures.push(name); console.error(`FAIL: ${name}: ${error.message}`); }
  }
  async function fits(width) {
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1),
      `No page-wide horizontal scrolling at ${width}px`);
  }
  async function visitOverview() {
    await page.goto(`${base}/admin/importers/harvest`);
    await expect(page.getByTestId('connection-status')).toHaveText('Connected');
  }
  async function visitRun() {
    await page.goto(`${base}/admin/importers/harvest/runs/${id}`);
    await expect(page.locator(`[data-run="${id}"]`)).toBeVisible();
  }
  const trigger = () => page.locator('#harvest-connection-trigger');
  async function openMenu() {
    await trigger().focus();
    await page.keyboard.press('Enter');
    await expect(trigger()).toHaveAttribute('aria-expanded', 'true');
  }
  async function keepsFocusInside(dialog) {
    assert.ok(await dialog.evaluate(el => el.contains(document.activeElement)), 'Focus moves into the dialog');
    for (let i = 0; i < 6; i++) {
      await page.keyboard.press('Tab');
      assert.ok(await dialog.evaluate(el => el.contains(document.activeElement) || document.activeElement === document.body),
        'Tab stays inside the dialog');
    }
  }
  // Hold the next account request until `release` is called.
  function hold() {
    let release;
    gate = new Promise(resolve => { release = resolve; });
    return release;
  }

  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);

    await check('the connection card offers changing account behind a guarded dialog', async () => {
      await visitOverview();
      await expect(page.getByText('fixture-account', { exact: true })).toBeVisible();
      await openMenu();
      await page.getByTestId('change-account').click();
      const dialog = page.getByRole('dialog', { name: 'Change Harvest account?' });
      await expect(dialog).toBeVisible();
      await expect(dialog).toContainText('releases account fixture-account');
      await keepsFocusInside(dialog);
      await page.keyboard.press('Escape');
      await expect(dialog).toBeHidden();
      assert.equal(changes, 0, 'Dismissing the dialog changes nothing');

      await openMenu();
      await page.getByTestId('change-account').click();
      const release = hold();
      try {
        await page.locator('#harvest-change-account-confirm').click();
        await expect(page.locator('#harvest-change-account-confirm')).toBeDisabled();
        await expect(page.locator('#harvest-change-account-confirm')).toHaveText('Changing account…');
        await expect(page.locator('#harvest-change-account-cancel')).toBeDisabled();
        await page.keyboard.press('Escape');
        await expect(dialog).toBeVisible();
      } finally { release(); }
      await expect(dialog.getByRole('alert')).toBeVisible();
      await expect(page.locator('#harvest-change-account-cancel')).toBeEnabled();
      assert.equal(changes, 1, 'One click sends one change');
      await page.locator('#harvest-change-account-cancel').click();
      await expect(dialog).toBeHidden();
    });

    await check('disconnecting keeps the dialog open while pending and shows its failure inside it', async () => {
      await visitOverview();
      await openMenu();
      await page.getByTestId('disconnect-harvest').click();
      const dialog = page.getByRole('dialog', { name: 'Disconnect Harvest?' });
      await expect(dialog).toBeVisible();
      await expect(dialog).toContainText('fixture-account');
      await keepsFocusInside(dialog);
      const release = hold();
      try {
        await page.locator('#harvest-disconnect-confirm').click();
        await expect(page.locator('#harvest-disconnect-confirm')).toHaveText('Disconnecting…');
        await expect(page.locator('#harvest-disconnect-cancel')).toBeDisabled();
        await page.keyboard.press('Escape');
        await expect(dialog).toBeVisible();
      } finally { release(); }
      await expect(dialog.getByRole('alert')).toContainText('Could not disconnect Harvest.');
      assert.equal(disconnects, 1);
      await page.keyboard.press('Escape');
      await expect(dialog).toBeHidden();
      await expect(page.getByTestId('connection-status')).toHaveText('Connected');
    });

    await check('an import in progress blocks changing account and disconnecting, with the reason', async () => {
      connection.active_imports = 1;
      await visitOverview();
      for (const [testid, dialog] of [['change-account', 'Change Harvest account?'], ['disconnect-harvest', 'Disconnect Harvest?']]) {
        // Choosing an item closes the menu, unavailable or not.
        if (await trigger().getAttribute('aria-expanded') !== 'true') await openMenu();
        const item = page.getByTestId(testid);
        await expect(item).toHaveAttribute('aria-disabled', 'true');
        await expect(item).toContainText('Unavailable: Finish or explicitly cancel');
        await item.click({ force: true });
        await expect(page.getByRole('dialog', { name: dialog })).toBeHidden();
      }
      assert.equal(changes + disconnects, 0);
    });

    await check('a CSV chosen from the keyboard opens its preview run with one primary action', async () => {
      await page.goto(`${base}/admin/importers/harvest/csv`);
      const file = page.getByTestId('csv-file');
      await file.focus();
      await expect(file).toBeFocused();
      await file.setInputFiles({ name: 'fixture.csv', mimeType: 'text/csv', buffer: Buffer.from('Date,Hours\n2026-09-15,1\n') });
      await expect(page.getByText('fixture.csv')).toBeVisible();
      await page.getByTestId('preview-csv').focus();
      await page.keyboard.press('Enter');
      await page.waitForURL(`${base}/admin/importers/harvest/runs/${id}`);
      await expect(page.getByTestId('commit-preview')).toBeVisible();
      await expect(page.getByTestId('run-badge')).toHaveText('Nothing imported yet');
      await expect(page.locator('main .btn-primary:visible')).toHaveCount(1);
      assert.equal(starts, 1, 'One preview starts');
    });

    await check('a report that cannot be read says so and keeps the run usable', async () => {
      selected = job('succeeded', 'Commit');
      selected.report = { version: 2, summary: 'unreadable' };
      await visitRun();
      await expect(page.getByText('Could not show the report. Download the errors or reload the run.')).toBeVisible();
      await expect(page.getByTestId('run-badge')).toHaveText('Imported');
    });

    await check('a stopped run from a previous account shows what it covered and the error download', async () => {
      selected = job('failed', 'Commit');
      selected.kind = 'harvest_api_import'; selected.source = 'harvest_api'; selected.retry_availability = 'previous_account';
      selected.report.source = 'harvest_api';
      selected.report.summary.time_entries.errored = 1;
      selected.report.row_errors = [{ entity: 'time_entry', source_location: 'row 2', reason: 'invalid date' }];
      summaryFailure = true;
      await visitRun();
      await expect(page.getByRole('heading', { name: 'What it covered before it stopped' })).toBeVisible();
      await expect(page.getByText('This import belongs to a previous Harvest account.', { exact: false })).toBeVisible();
      await expect(page.getByTestId('retry-run')).toHaveCount(0);
      await expect(page.getByText("Could not group this run's errors.", { exact: false })).toBeVisible();
      await expect(page.getByRole('link', { name: 'Download all errors (NDJSON)' }))
        .toHaveAttribute('href', `/api/import/harvest/jobs/${id}/errors`);
    });

    await check('records that could not keep their Harvest id are listed with their new id', async () => {
      selected = job('succeeded', 'Commit');
      selected.kind = 'harvest_api_import'; selected.source = 'harvest_api';
      selected.report.source = 'harvest_api';
      selected.report.summary.clients.created = 1;
      selected.report.id_notices = [{ entity: 'client', harvest_id: 70001, api_id: 12 }];
      selected.report.omitted_id_notices = 3;
      await visitRun();
      const group = page.locator('details', { hasText: 'Records with a new id in the Harvest-compatible API' });
      await expect(group).toBeVisible();
      await group.locator('summary').click();
      const row = group.getByRole('row', { name: /70001/ });
      await expect(row).toContainText('Clients');
      await expect(row).toContainText('12');
      await expect(group.getByText('3 more are only counted.')).toBeVisible();
    });

    await check('a failed status read checks the same run again without starting another', async () => {
      selected = job('running'); statusFailure = true;
      await visitRun();
      await expect(page.getByText('Status unavailable. The import may still be running.')).toBeVisible();
      statusFailure = false;
      await page.getByTestId('resume-monitoring').click();
      await expect(page.getByTestId('run-badge')).toHaveText('Nothing in Horae changes');
      assert.ok(statusReads.length >= 2, 'The run is read again');
      assert.ok(statusReads.every(value => value === id), 'Only this run is read');
      assert.equal(starts, 0, 'No import starts');
    });

    await check('long identifiers wrap on the card, the run and the dialogs at every width', async () => {
      connection.account_id = longId;
      selected = job('succeeded', 'Commit'); history = [selected];
      selected.report.summary.time_entries.errored = 1;
      selected.report.row_errors = [{ entity: 'time_entry', source_location: longId, reason: 'invalid date' }];
      for (const width of [360, 768, 1440]) {
        await page.setViewportSize({ width, height: 900 });
        await visitRun();
        await expect(page.getByRole('cell', { name: longId })).toBeVisible();
        await fits(width);
        await visitOverview();
        await expect(page.getByText(longId, { exact: true })).toBeVisible();
        await fits(width);
        for (const [testid, name] of [['change-account', 'Change Harvest account?'], ['disconnect-harvest', 'Disconnect Harvest?']]) {
          await openMenu();
          await page.getByTestId(testid).click();
          const dialog = page.getByRole('dialog', { name });
          await expect(dialog).toBeVisible();
          assert.ok(await dialog.locator('.modal').evaluate(el => el.scrollWidth <= el.clientWidth),
            `${name} wraps the account at ${width}px`);
          await fits(width);
          await page.keyboard.press('Escape');
          await expect(dialog).toBeHidden();
        }
      }
    });
    assert.deepEqual(errors, [], 'No browser runtime errors');
    assert.deepEqual(failures, [], 'Every browser scenario must pass');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
