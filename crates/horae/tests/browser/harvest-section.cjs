// The Harvest section's pages, reached and reloaded through their own URLs.
// Run only against an isolated, seeded dev-login instance. Every importer and
// people request is intercepted, so no import or account reaches the server.
// HORAE_TEST_URL=http://127.0.0.1:8093 node crates/horae/tests/browser/harvest-section.cjs
// HORAE_SCREENSHOT_DIR, when set, receives one screenshot per page and width.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const path = require('node:path');
const base = process.env.HORAE_TEST_URL;
const shots = process.env.HORAE_SCREENSHOT_DIR;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated fixture instance');
assert.ok(['127.0.0.1', 'localhost'].includes(new URL(base).hostname), 'Fixtures require a loopback instance');

const previewId = '019956ed-0000-7000-8000-000000000001';
const commitId = '019956ed-0000-7000-8000-000000000002';
const counts = (errored = 0) => ({ created: 3, updated: 0, skipped: 1, errored });
const report = mode => ({
  version: 2, source: 'csv', mode,
  summary: { clients: counts(), projects: counts(), tasks: counts(), people: counts(), time_entries: counts(4) },
  row_errors: [], error_archive: { count: 4, chunks: 1 },
});
const run = (id, mode, status = 'succeeded') => ({
  id, kind: 'harvest_csv_import', status, phase: status === 'running' ? 'time_entries' : null, error_kind: null,
  processed_count: 12, total_count: null, report: status === 'queued' ? null : report(mode), last_error: null,
  created_at: '2026-09-15T10:30:00Z', finished_at: null, retry_availability: 'unavailable_state',
  mode, source: 'csv', commit_availability: mode === 'DryRun' ? 'available' : 'not_preview',
});
const summary = {
  groups: [
    { entity: 'time_entry', kind: 'unmatched_email', entry_kind: null, reason: 'no Horae user matches email "dana@example.com"', count: 3,
      samples: ['row 2', 'row 3', 'row 4'].map(location => ({ location, reason: 'no Horae user matches email "dana@example.com"' })) },
    { entity: 'time_entry', kind: null, entry_kind: 'negative_hours', reason: 'negative hours: -0.5', count: 1,
      samples: [{ location: 'row 9', reason: 'negative hours: -0.5' }] },
  ],
  people: [
    { email: 'dana@example.com', name: 'Dana Dev', kind: 'unmatched_email', entries: 3 },
    { email: 'sam@example.com', name: 'Sam Long-Named Person With A Very Long Name', kind: 'unmatched_email', entries: 1 },
    { email: null, name: 'No Mail', kind: 'missing_email', entries: 2 },
  ],
  omitted_errors: 0, omitted_people: 0,
};
// `add_harvest_people` answers with list rows, not whole accounts.
const user = email => ({ id: '019956ed-0000-7000-8000-0000000000a0', email, name: email, active: false });

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const failures = [];
  const errors = [];
  let commits, additions, summaries, statusReads;
  page.on('pageerror', error => {
    // Leaving a page that still follows a running job aborts its status poll.
    // Match the name: Firefox and Chromium word the message differently.
    if (error.name === 'AbortError') return;
    errors.push(error.message);
  });
  const reset = () => { commits = 0; additions = []; summaries = 0; statusReads = 0; };
  reset();
  // Only API calls: the section's own page URLs contain "harvest" too.
  await page.route(url => url.pathname.startsWith('/api/') && url.pathname.includes('harvest'), async route => {
    const request = route.request();
    const pathname = new URL(request.url()).pathname;
    const reply = value => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(value) });
    if (pathname === '/api/import/harvest/connection') {
      return reply({ configured: true, connected: true, account_id: 'fixture-account', token_expired: false,
        account_generation: 2, connection_revision: 4, has_provenance: false, active_imports: 0 });
    }
    if (pathname === '/api/import/harvest/history') return reply([run(commitId, 'Commit', 'queued'), run(previewId, 'DryRun')]);
    if (pathname === '/api/import/harvest/status') {
      statusReads++;
      const { job_id } = request.postDataJSON();
      return reply(job_id === commitId ? run(commitId, 'Commit', 'queued') : run(previewId, 'DryRun'));
    }
    if (pathname === '/api/import/harvest/error-summary') { summaries++; return reply(summary); }
    if (pathname === '/api/import/harvest/commit-preview') { commits++; return reply(run(commitId, 'Commit', 'queued')); }
    if (pathname.includes('add_harvest_people')) {
      const { people, activation } = request.postDataJSON();
      additions.push({ emails: people.map(person => person.email), activation });
      // Sam was added from another tab meanwhile.
      const present = people.filter(person => person.email === 'sam@example.com');
      const added = people.filter(person => !present.includes(person));
      return reply({
        added: added.map(person => user(person.email)),
        present: present.map(person => person.email),
        unavailable: [],
      });
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
    for (const element of await page.locator('main button:visible, main a.btn:visible').all()) {
      const box = await element.boundingBox();
      assert.ok(box.x >= -1 && box.x + box.width <= width + 1, `Controls fit horizontally at ${width}px`);
    }
  }
  async function shot(name) {
    if (shots) await page.screenshot({ path: path.join(shots, `${name}.png`), fullPage: true });
  }
  const runUrl = id => `${base}/admin/importers/harvest/runs/${id}`;
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);

    await check('every page has its own URL, from the index to one run', async () => {
      await page.goto(`${base}/admin/importers`);
      await expect(page.getByTestId('choose-api')).toBeVisible();
      await expect(page.getByText('Toggl Track')).toHaveCount(0);
      await expect(page.getByText('Clockify')).toHaveCount(0);
      await page.getByTestId('choose-api').click();
      await page.waitForURL(/\/admin\/importers\/harvest\??$/);
      await expect(page.getByRole('heading', { level: 1, name: 'Harvest' })).toBeVisible();
      await expect(page.locator('.adm-link.active')).toHaveText('Importers');
      await page.getByTestId('all-runs').click();
      await page.waitForURL(/\/admin\/importers\/harvest\/runs\??$/);
      await page.getByTestId(`run-${previewId}`).click();
      await page.waitForURL(runUrl(previewId));
      await expect(page.getByText('15 records are ready')).toBeVisible();
      await page.reload();
      await expect(page.getByText('15 records are ready')).toBeVisible();
      await expect(page.getByTestId('commit-preview')).toBeVisible();
    });

    await check('a run groups its errors and lists the people to add', async () => {
      await page.goto(runUrl(previewId));
      const errorsTable = page.getByRole('region', { name: 'Needs your attention' });
      await expect(errorsTable.getByText('Time entries: No Horae account has this email')).toBeVisible();
      // The first group is open and lists its rows; the others open on demand.
      await expect(errorsTable.getByRole('cell', { name: 'row 4' })).toBeVisible();
      await errorsTable.getByText('Time entries with negative hours, skipped').click();
      await expect(errorsTable.getByRole('cell', { name: 'negative hours: -0.5' })).toBeVisible();
      await expect(errorsTable.getByRole('link', { name: 'Download every issue (NDJSON)' }))
        .toHaveAttribute('href', `/api/import/harvest/jobs/${previewId}/errors`);
      const people = page.getByRole('region', { name: 'People not imported' });
      await expect(people.getByText('dana@example.com')).toBeVisible();
      await expect(people.getByText('Add an email to this person in Harvest', { exact: false })).toBeVisible();
      assert.equal(summaries, 1, 'A finished run groups its errors once');
      // Coming back to the tab re-reads the finished run, and only the run.
      const before = statusReads;
      await page.evaluate(() => window.dispatchEvent(new Event('focus')));
      await expect.poll(() => statusReads).toBeGreaterThan(before);
      assert.equal(summaries, 1, 'Re-reading the run does not regroup its errors');
    });

    await check('a person is added in place, deactivated by default', async () => {
      await page.goto(runUrl(previewId));
      await page.getByTestId('add-person-dana@example.com').click();
      assert.ok(await page.getByTestId('person-email-dana@example.com').evaluate(input => input.readOnly),
        'A Harvest email is shown, not edited');
      await page.getByTestId('confirm-person-dana@example.com').click();
      await expect(page.getByText('Added 1 people', { exact: false })).toBeVisible();
      assert.deepEqual(additions, [{ emails: ['dana@example.com'], activation: 'Deactivated' }]);
      await page.getByTestId('add-all-people').click();
      await expect(page.getByText('1 already had an account', { exact: false })).toBeVisible();
      // The entity table has an "Already in Horae" column too.
      await expect(page.getByRole('region', { name: 'People not imported' }).getByText('Already in Horae')).toBeVisible();
      assert.deepEqual(additions[1], { emails: ['sam@example.com'], activation: 'Deactivated' });
      await expect(page.getByTestId('preview-again')).toHaveAttribute('href', '/admin/importers/harvest/csv');
    });

    await check('committing a dry run opens the commit run', async () => {
      await page.goto(runUrl(previewId));
      await page.getByTestId('commit-preview').click();
      await page.waitForURL(runUrl(commitId));
      await expect(page.getByText('Waiting to start…')).toBeVisible();
      await expect(page.getByTestId('run-badge')).toHaveText('Writing to Horae');
      assert.equal(commits, 1);
    });

    await check('each page fits phone, tablet and desktop widths', async () => {
      for (const width of [360, 768, 1440]) {
        await page.setViewportSize({ width, height: 900 });
        for (const [name, url] of [
          ['index', `${base}/admin/importers`],
          ['overview', `${base}/admin/importers/harvest`],
          ['csv', `${base}/admin/importers/harvest/csv`],
          ['runs', `${base}/admin/importers/harvest/runs`],
          ['run', runUrl(previewId)],
        ]) {
          await page.goto(url);
          await expect(page.locator('main h1')).toBeVisible();
          if (name === 'run') await expect(page.getByRole('region', { name: 'People not imported' })).toBeVisible();
          if (name === 'overview' || name === 'runs') await expect(page.getByTestId(`run-${previewId}`)).toBeVisible();
          await fits(width);
          await shot(`${name}-${width}`);
        }
      }
    });
    assert.deepEqual(errors, [], 'No browser runtime errors');
    assert.deepEqual(failures, [], 'Every browser scenario must pass');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
