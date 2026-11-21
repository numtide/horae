// The People screen: the week's hours, the week pager, filters, the row menu
// and its states, as an administrator and as someone who may only read people.
// With HORAE_BROWSER_ARTIFACTS it saves 1280px dark screenshots of each state.
// This suite changes only the database created by run-design-checks.sh.
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { expect } = playwright;
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { setAsideSeededStates } = require('./seeded-permissions.cjs');
const { mkdirSync } = require('node:fs');
const { join } = require('node:path');
const evidence = process.env.HORAE_BROWSER_ARTIFACTS;
if (evidence) mkdirSync(evidence, { recursive: true });
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id,name FROM users WHERE email='admin@example.com' AND active) u"));
const state = '019f4500-0000-7000-8000-000000000002';
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const array = values => `ARRAY[${values.map(value => `'${value}'`).join(',')}]`;
let restoreSeededStates = () => {};

(async () => {
  const browser = await playwright.chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 }, colorScheme: 'dark' });
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const shot = async name => {
    if (!evidence) return;
    await page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
    await page.screenshot({ path: join(evidence, `people-${name}-1280-dark.png`), animations: 'disabled' });
  };
  const row = name => page.locator('tbody tr').filter({ has: page.getByText(name, { exact: true }) });
  const summary = page.getByRole('region', { name: 'Week summary', exact: true });
  const content = page.locator('.ppl-content');
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/people`);

    // The seeded week has time, so the administrator sees everyone's hours.
    await expect(row(actor.name)).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add person', exact: true })).toBeVisible();
    await expect(summary).toBeVisible();
    const thisWeek = await summary.textContent();
    assert.doesNotMatch(thisWeek, /Total hours0:00/);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false);
    await shot('week-admin');

    // A new week keeps the rows, dimmed, until its hours arrive.
    let release;
    const held = new Promise(resolve => { release = resolve; });
    await page.route('**/api/list_people*', async route => { await held; await route.continue(); });
    await page.locator('#people-week-next').click();
    await expect(content).toHaveAttribute('aria-busy', 'true');
    await expect(row(actor.name)).toBeVisible();
    release();
    await expect(content).not.toHaveAttribute('aria-busy', 'true');
    await expect(summary).toContainText('Total hours0:00');
    await page.unroute('**/api/list_people*');
    await page.getByRole('button', { name: 'This week', exact: true }).click();
    await expect(summary).toHaveText(thisWeek);
    console.log('PASS: changing the week keeps the rows dimmed until the new hours arrive');

    // The row menu of an administrator offers every action, Archive last.
    await row(actor.name).getByRole('button', { name: 'Actions', exact: true }).click();
    const items = page.locator('.menu-popover:popover-open [role="menuitem"]');
    await expect(items).toHaveText(['View timesheet', 'Permissions…', 'Rates…', 'Permission history', 'Archive']);
    await shot('row-menu');
    await items.filter({ hasText: 'View timesheet' }).click();
    await page.waitForURL(url => url.pathname.startsWith('/timesheet/week/') && url.searchParams.get('user') === actor.id);
    await page.goto(`${base}/people`);

    // Add person is a dialog with the profile as radio cards.
    await page.getByRole('button', { name: 'Add person', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Add person', exact: true });
    await expect(dialog.getByRole('radio')).toHaveCount(5);
    await shot('add-person');
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(dialog).not.toBeVisible();

    // A search that matches nobody says so and offers to clear it.
    await page.getByRole('searchbox', { name: 'Search by name or email', exact: true }).fill('zzz');
    await expect(page.getByRole('heading', { name: 'No one matches “zzz”', exact: true })).toBeVisible();
    await shot('no-results');
    await page.getByRole('button', { name: 'Clear search', exact: true }).click();
    await expect(row(actor.name)).toBeVisible();

    // The first load shows a skeleton under a live header and pager.
    let first;
    const slow = new Promise(resolve => { first = resolve; });
    await page.route('**/api/list_people*', async route => { await slow; await route.continue(); });
    await page.reload();
    await expect(page.locator('.skeleton')).toBeVisible();
    await expect(page.locator('#people-week-next')).toBeVisible();
    await shot('loading');
    first();
    await expect(row(actor.name)).toBeVisible();
    await page.unroute('**/api/list_people*');
    console.log('PASS: the administrator sees the menu, Add person, no results and loading states');

    // Someone who may read people but not manage them, nor read their time.
    restoreSeededStates = setAsideSeededStates(actor.org_id);
    sql(`INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
      VALUES ('${state}','${actor.org_id}','${actor.id}',1,${array([...floor, 'people_read_all'])},false,'individual')`);
    await page.reload();
    await expect(row(actor.name)).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add person', exact: true })).toHaveCount(0);
    // Their own time is theirs to read; nobody else's is.
    const others = page.locator('tbody tr').filter({ hasNot: page.getByText(actor.name, { exact: true }) });
    await expect(others.first().getByLabel('Hours not available', { exact: true })).toBeVisible();
    await expect(others.first().getByRole('button', { name: 'Actions', exact: true })).toHaveCount(0);
    await row(actor.name).getByRole('button', { name: 'Actions', exact: true }).click();
    await expect(items).toHaveText(['View timesheet']);
    await page.keyboard.press('Escape');
    await shot('read-only');
    assert.deepEqual(errors, []);
    console.log('PASS: a people reader sees only their own hours and no management actions');
  } finally {
    await browser.close();
    sql(`DELETE FROM person_permission_states WHERE id='${state}'`);
    restoreSeededStates();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
