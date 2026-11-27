// Run against an isolated, seeded dev-login instance with Playwright installed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/concurrent-reads.cjs
//
// Pages start their reads together: each holds back its first read and
// checks that the others are already on their way. The version check asks on
// mount and when the tab is shown, not on navigation, and loading states
// appear only after a short wait.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push({ message: error.message, stack: error.stack, url: page.url() }));
  // Server-function calls by name, in the order they started.
  let calls = [];
  page.on('request', request => {
    const match = new URL(request.url()).pathname.match(/^\/api\/([a-z_]+)/);
    if (request.method() === 'POST' && match) calls.push(match[1]);
  });
  const sidebar = page.locator('.app-sidebar');
  const go = name => sidebar.getByRole('link', { name, exact: true }).click();
  // Holds `held` until the returned release runs, recording the calls that
  // started meanwhile.
  const hold = async held => {
    let release;
    const gate = new Promise(resolve => { release = resolve; });
    let seen = null;
    const handler = async route => { await gate; await route.fallback(); };
    await page.route(`**/api/${held}*`, handler);
    return {
      started: () => { seen = [...calls]; return seen; },
      release: async () => { release(); await page.unroute(`**/api/${held}*`, handler, { behavior: 'wait' }); },
    };
  };
  const startedWhileHeld = async (held, action, expected) => {
    const gate = await hold(held);
    calls = [];
    await action();
    await expect.poll(() => gate.started()).toEqual(expect.arrayContaining([held, ...expected]));
    await gate.release();
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    const ready = page.waitForResponse(r => r.url().includes('/api/load_timesheet_tracking') && r.status() === 200);
    await page.reload();
    await (await ready).finished();

    // Projects: tags, spend and budget progress start beside the overview.
    await startedWhileHeld('get_project_overview', () => go('Projects'),
      ['list_project_tags', 'list_project_spend', 'list_project_budget_progress']);
    await expect(page.locator('#project-list-heading')).toBeVisible();
    console.log('PASS: Projects starts its auxiliary reads beside the overview');

    // Reports: the session's grants choose the view, with no access round.
    calls = [];
    await go('Reports');
    await expect(page.getByRole('link', { name: 'Export CSV', exact: true })).toBeVisible();
    assert.ok(!calls.includes('get_time_report_access'), calls.join(', '));
    console.log('PASS: Reports decides its view from the session');

    // Timesheet: the tracking choices and the person picker start beside the sheet.
    await startedWhileHeld('load_timesheet_page', () => go('Timesheet'),
      ['load_timesheet_tracking', 'list_timesheet_people']);
    await expect(page.locator('#timesheet-person')).toBeVisible();
    console.log('PASS: Timesheet loads its choices and person picker beside the sheet');

    // Client detail: its projects, totals and invoices start beside the view.
    await go('Clients');
    const client = page.locator('main a[href^="/clients/"]').first();
    await expect(client).toBeVisible();
    await startedWhileHeld('client_detail_view', () => client.click(),
      ['list_projects', 'list_project_spend', 'list_client_invoices']);
    await expect(page.locator('#client-projects-title')).toBeVisible();
    console.log('PASS: client detail starts its sections beside the view');

    // Invoices: the recovery gate reads the identity and permissions together.
    await startedWhileHeld('get_me', () => go('Invoices'), ['get_my_permissions']);
    await expect(page.getByRole('heading', { name: 'Invoices', exact: true })).toBeVisible();
    console.log('PASS: the invoice gate reads identity and permissions together');

    // People: no identity read until a person is opened; then the person
    // list starts beside the editor.
    calls = [];
    await go('People');
    const menu = page.locator('button[aria-label^="Actions for "]').first();
    await expect(menu).toBeVisible();
    assert.ok(!calls.includes('get_me'), calls.join(', '));
    await menu.click();
    await startedWhileHeld('load_permission_editor',
      () => page.locator('.menu-popover:popover-open [role="menuitem"]').filter({ hasText: 'Permissions…' }).click(),
      ['get_me', 'list_permission_subjects']);
    const editor = page.getByRole('dialog', { name: 'Edit permissions', exact: true });
    await expect(editor.getByRole('button', { name: 'Change person' })).toBeEnabled();
    await page.keyboard.press('Escape');
    await expect(editor).not.toBeVisible();
    console.log('PASS: People reads the identity only to open a person, and the picker loads beside the editor');

    // The version check: not on navigation, again when the tab is shown.
    calls = [];
    await go('Projects');
    await expect(page.locator('#project-list-heading')).toBeVisible();
    await go('Clients');
    await expect(page.locator('main a[href^="/clients/"]').first()).toBeVisible();
    assert.ok(!calls.includes('get_server_version'), calls.join(', '));
    const asked = page.waitForRequest(r => r.url().includes('/api/get_server_version'));
    await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
    await asked;
    // Within the minute, coming back again does not ask again.
    calls = [];
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await page.waitForTimeout(500);
    assert.ok(!calls.includes('get_server_version'), calls.join(', '));
    console.log('PASS: the version check asks when the tab is shown, at most once a minute, not on navigation');

    // A loading state is in place at once but drawn only after a short wait.
    const gate = await hold('client_overview');
    await go('Projects');
    await expect(page.locator('#project-list-heading')).toBeVisible();
    await go('Clients');
    const loading = page.locator('main .loading, main .skeleton, main .table-skeleton').first();
    await expect(loading).toBeAttached();
    assert.equal(await loading.evaluate(el => getComputedStyle(el).visibility), 'hidden');
    await expect(loading).toBeVisible();
    await gate.release();
    await expect(page.locator('main a[href^="/clients/"]').first()).toBeVisible();
    console.log('PASS: loading states appear only after a short wait');

    // Reports arrives with the page. On a cold load its dates wait, disabled,
    // for hydration, so a date typed at once is kept rather than replaced.
    const cold = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
    assert.ok((await cold.request.post(`${base}/auth/dev-login`, { maxRedirects: 0 })).status() < 400);
    const coldPage = await cold.newPage();
    coldPage.on('pageerror', error => errors.push({ message: error.message, stack: error.stack, url: coldPage.url() }));
    let wasm;
    const wasmHeld = new Promise(resolve => { wasm = resolve; });
    await coldPage.route('**/*.wasm*', async route => { await wasmHeld; await route.fallback(); });
    const queried = [];
    coldPage.on('request', request => {
      if (request.url().includes('/api/report_time')) queried.push(JSON.parse(request.postData() || '{}').from);
    });
    await coldPage.goto(`${base}/reports`, { waitUntil: 'domcontentloaded' });
    const from = coldPage.locator('#report-filter-from');
    await expect(from).toBeDisabled();
    const typing = from.fill('2020-01-01');
    wasm();
    await typing;
    await expect.poll(() => queried).toContain('2020-01-01');
    await coldPage.waitForTimeout(1000);
    await expect(from).toHaveValue('2020-01-01');
    await cold.close();
    console.log('PASS: a date typed during a cold Reports load is kept');

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
})().catch(error => {
  console.error(error);
  process.exit(1);
});
