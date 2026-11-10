// Run only against an isolated, seeded dev-login instance with no Harvest credentials.
// All importer/provider requests are intercepted; no import or account mutation reaches the server.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/importers.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const path = require('node:path');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated fixture instance');
assert.ok(['127.0.0.1', 'localhost'].includes(new URL(base).hostname), 'Fixtures require a loopback instance');
const id = '019956ed-0000-7000-8000-000000000001';
const counts = () => ({ created: 0, updated: 0, skipped: 0, errored: 0 });
const report = (mode = 'DryRun') => ({
  version: 1, source: 'csv', mode,
  summary: { clients: counts(), projects: counts(), tasks: counts(), time_entries: counts() }, row_errors: [],
});
const job = (status = 'succeeded', mode = 'DryRun') => ({
  id, kind: 'harvest_csv_import', status, phase: 'time_entries', processed_count: 12,
  total_count: null, report: report(mode), last_error: null,
  created_at: '2026-09-15T10:30:00Z', finished_at: null, retry_availability: 'unavailable_state',
  mode, source: 'csv', commit_availability: mode === 'DryRun' && status === 'succeeded' ? 'available' : 'not_preview',
});
const connected = () => ({ configured: true, connected: true, account_id: 'fixture-account',
  token_expired: false, account_generation: 2, connection_revision: 4, has_provenance: false, active_imports: 0 });
const groups = errors => ({
  groups: errors.map(error => ({ entity: 'time_entry', kind: null, reason: error.reason, count: 1, samples: [error.source_location] })),
  people: [], omitted_errors: 0, omitted_people: 0,
});

(async () => {
  const zoomExtension = path.join(__dirname, 'fixtures', 'zoom');
  const context = await chromium.launchPersistentContext('', {
    executablePath: process.env.CHROMIUM_PATH || undefined, headless: true,
    viewport: { width: 1440, height: 640 },
    args: [`--disable-extensions-except=${zoomExtension}`, `--load-extension=${zoomExtension}`],
  });
  const page = await context.newPage();
  const failures = [];
  const errors = [];
  let connection, history, selected, statusFailure, changeGate, starts, changes, statusReads;
  page.on('pageerror', error => errors.push(error.message));
  function reset() {
    connection = connected(); history = []; selected = job(); statusFailure = false;
    changeGate = null; starts = 0; changes = 0; statusReads = [];
  }
  reset();
  // Only API calls: the section's own page URLs contain "harvest" too.
  await page.route(url => url.pathname.startsWith('/api/') && url.pathname.includes('harvest'), async route => {
    const path = new URL(route.request().url()).pathname;
    const reply = value => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(value) });
    if (path === '/api/import/harvest/connection') return reply(connection);
    if (path.includes('harvest_change_account')) {
      changes++;
      if (changeGate) await changeGate;
      return route.abort('failed');
    }
    if (path.includes('harvest_connect_start') || path.includes('harvest_disconnect')) return route.abort('failed');
    if (path === '/api/import/harvest/history') return reply(history);
    if (path === '/api/import/harvest/status') {
      statusReads.push(route.request().postDataJSON().job_id);
      if (statusFailure) return route.abort('failed');
      return reply(selected);
    }
    if (path === '/api/import/harvest/error-summary') return reply(groups(selected.report.row_errors));
    if (path.startsWith('/api/import/harvest/csv-job/') || path === '/api/import/harvest/start') {
      starts++;
      return reply({ ...selected, status: 'queued' });
    }
    if (path === `/api/import/harvest/jobs/${id}/errors`) {
      return route.fulfill({ contentType: 'application/x-ndjson', body: '{"reason":"fixture"}\n' });
    }
    errors.push(`Unrecognized importer request was blocked: ${path}`);
    return route.abort('failed');
  });
  async function visit() {
    const loaded = page.waitForResponse(r => r.url().includes('/api/import/harvest/history'));
    await page.goto(`${base}/admin/importers/harvest`);
    await loaded;
    await expect(page.getByTestId('all-importers')).toBeVisible();
  }
  async function visitRun() {
    await page.goto(`${base}/admin/importers/harvest/runs/${id}`);
    await expect(page.getByTestId('back-to-harvest')).toBeVisible();
  }
  async function check(name, run) {
    try { reset(); await run(); console.log(`PASS: ${name}`); }
    catch (error) { failures.push(name); console.error(`FAIL: ${name}: ${error.message}`); }
  }
  async function fits(width) {
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1), `No page-wide horizontal scrolling at ${width}px`);
    for (const element of await page.locator('main button:visible, main input:visible').all()) {
      const box = await element.boundingBox();
      assert.ok(box.x >= -1 && box.x + box.width <= width + 1, 'Essential controls fit horizontally');
    }
  }
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);

    await check('connection disclosure, modal focus and pending safeguards', async () => {
      await visit();
      const manage = page.getByTestId('manage-connection');
      await expect(manage).toHaveAttribute('aria-expanded', 'false');
      await manage.focus(); await page.keyboard.press('Enter');
      await expect(manage).toHaveAttribute('aria-expanded', 'true');
      const trigger = page.getByTestId('change-account');
      const modal = page.getByRole('dialog', { name: 'Change Harvest account?' });
      await trigger.click();
      await expect(modal).toBeVisible();
      assert.ok(await modal.evaluate(el => el.contains(document.activeElement)), 'Focus moves into dialog');
      for (let i = 0; i < 8; i++) {
        await page.keyboard.press('Tab');
        assert.ok(await modal.evaluate(el => el.contains(document.activeElement) || document.activeElement === document.body));
      }
      await page.keyboard.press('Escape');
      await expect(modal).toBeHidden(); await expect(trigger).toBeFocused();
      assert.equal(changes, 0);
      await trigger.click();
      let release;
      changeGate = new Promise(resolve => { release = resolve; });
      try {
        await page.getByTestId('confirm-change-account').click();
        await expect(page.getByTestId('confirm-change-account')).toBeDisabled();
        await expect(page.getByTestId('cancel-change-account')).toBeDisabled();
        await page.keyboard.press('Escape');
        await expect(modal).toBeVisible();
      } finally { release(); }
      await expect(modal.getByRole('alert')).toBeVisible();
      assert.equal(changes, 1);
      await page.keyboard.press('Escape');
    });

    await check('keyboard CSV choice opens the preview run with a single primary action', async () => {
      await page.goto(`${base}/admin/importers/harvest/csv`);
      const file = page.getByTestId('csv-file');
      await file.focus(); await expect(file).toBeFocused();
      await file.setInputFiles({ name: 'fixture.csv', mimeType: 'text/csv', buffer: Buffer.from('Date,Hours\n2026-09-15,1\n') });
      await page.getByTestId('preview-csv').click();
      await page.waitForURL(`${base}/admin/importers/harvest/runs/${id}`);
      await expect(page.getByTestId('commit-preview')).toBeVisible();
      await expect(page.locator('main .btn-primary:visible')).toHaveCount(1);
      await expect(page.getByText('No business data was written.', { exact: false })).toBeVisible();
      assert.equal(starts, 1);
      await page.getByTestId('back-to-harvest').click();
      await expect(page.getByTestId('commit-preview')).toHaveCount(0);
    });

    await check('old-account partial report explains the missing prerequisite and keeps the download', async () => {
      selected = job('failed', 'Commit');
      selected.kind = 'harvest_api_import'; selected.source = 'harvest_api'; selected.retry_availability = 'previous_account';
      selected.report.source = 'harvest_api';
      selected.report.summary.time_entries.errored = 1;
      selected.report.row_errors = [{ entity: 'time_entry', source_location: 'row 2', reason: 'invalid date' }];
      await visitRun();
      await expect(page.getByText('Partial report — confirmed batches only')).toBeVisible();
      await expect(page.getByText('previous Harvest account', { exact: false })).toBeVisible();
      await expect(page.getByTestId('retry-run')).toHaveCount(0);
      await expect(page.getByText('invalid date')).toBeVisible();
      await expect(page.getByRole('link', { name: 'Download all errors (NDJSON)' })).toBeVisible();
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

    await check('monitoring failure checks the same ID again without a new import', async () => {
      selected = job('running'); statusFailure = true;
      await visitRun(); await expect(page.getByTestId('resume-monitoring')).toBeVisible();
      statusFailure = false;
      await page.getByTestId('resume-monitoring').click();
      await expect(page.getByText('Leaving this page does not stop the import.')).toBeVisible();
      assert.ok(statusReads.length >= 2);
      assert.ok(statusReads.every(value => value === id));
      assert.equal(starts, 0);
    });

    await check('connection, long identifiers, result tiles and modal fit the viewport matrix', async () => {
      connection.account_id = 'fixture'.repeat(40);
      selected = job('succeeded', 'Commit'); history = [selected];
      selected.report.summary.time_entries.errored = 1;
      selected.report.row_errors = [{ entity: 'time_entry', source_location: 'fixture'.repeat(60), reason: 'invalid date' }];
      for (const width of [360, 768, 1440]) {
        await page.setViewportSize({ width, height: 640 });
        await visitRun();
        await expect(page.getByText('Import complete with 1 errors', { exact: true })).toBeVisible();
        await fits(width);
        await visit(); await fits(width);
        await page.getByTestId('manage-connection').click();
        await page.getByTestId('change-account').click();
        const modal = page.getByRole('dialog', { name: 'Change Harvest account?' });
        const panel = modal.locator('.modal');
        await expect(modal).toBeVisible();
        assert.ok(await panel.evaluate(el => el.scrollWidth <= el.clientWidth), 'Modal content wraps');
        await fits(width);
        await page.keyboard.press('Escape');
      }
    });

    await check('actual 200% browser zoom retains usable importer controls', async () => {
      const worker = context.serviceWorkers()[0] || await context.waitForEvent('serviceworker');
      await page.setViewportSize({ width: 1440, height: 640 });
      history = [selected];
      await visit();
      const before = await page.evaluate(() => ({ width: window.innerWidth, ratio: window.devicePixelRatio }));
      const zoom = await worker.evaluate(async base => {
        const tabs = await chrome.tabs.query({});
        const tab = tabs.find(tab => tab.url?.startsWith(`${base}/admin/importers`));
        if (!tab) throw new Error('No importer fixture tab');
        await chrome.tabs.setZoom(tab.id, 2);
        return chrome.tabs.getZoom(tab.id);
      }, base);
      assert.equal(zoom, 2, 'Browser reports real 200% page zoom');
      await expect.poll(() => page.evaluate(() => window.innerWidth)).toBeLessThan(before.width);
      const after = await page.evaluate(() => ({ width: window.innerWidth, ratio: window.devicePixelRatio }));
      assert.ok(after.ratio >= before.ratio * 1.99, 'Page zoom changes effective pixel ratio');
      await fits(after.width);
      await page.getByTestId('manage-connection').click();
      await page.getByTestId('change-account').click();
      const modal = page.getByRole('dialog', { name: 'Change Harvest account?' });
      await expect(modal).toBeVisible();
      assert.ok(await modal.locator('.modal').evaluate(el => el.scrollWidth <= el.clientWidth));
      await expect(page.getByTestId('cancel-change-account')).toBeInViewport();
      await page.keyboard.press('Escape');
      await page.getByTestId(`run-${id}`).click();
      await fits(after.width);
      console.log(`ZOOM: ${zoom * 100}%, layout ${before.width}px → ${after.width}px, DPR ${before.ratio} → ${after.ratio}`);
    });
    assert.deepEqual(errors, [], 'No browser runtime errors');
    assert.deepEqual(failures, [], 'Every browser scenario must pass');
  } finally { await context.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
