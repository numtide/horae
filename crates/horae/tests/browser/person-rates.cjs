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
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id FROM users WHERE email='admin@example.com' AND active) u"));
const person = '019f4400-0000-7000-8000-000000000001';
const state = '019f4400-0000-7000-8000-000000000002';
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const array = values => `ARRAY[${values.map(value => `'${value}'`).join(',')}]`;
const stored = () => sql(`SELECT concat_ws('/', coalesce(billable_rate_cents::text,'-'), coalesce(cost_rate_cents::text,'-')) FROM users WHERE id='${person}'`);
let restoreSeededStates = () => {};

(async () => {
  // Chromium in CI; HORAE_BROWSER=firefox for local evidence runs.
  const name = process.env.HORAE_BROWSER || 'chromium';
  const browser = await playwright[name].launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  // Firefox reports a fetch cut short by page.goto as a page error; Chromium does not.
  const aborted = message => name === 'firefox' && message === 'NetworkError when attempting to fetch resource.';
  page.on('pageerror', error => { if (!aborted(error.message)) errors.push(error.message); });
  try {
    // The People page reads its own access only once the organization is on
    // the stored permissions; the seeded Administrator state stays.
    sql(`BEGIN; INSERT INTO users (id,org_id,email,name,billable_rate_cents)
      VALUES ('${person}','${actor.org_id}','rates-person@example.com','Rates Person',9000); COMMIT;`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    // The Administrator profile edits both rates.
    await page.goto(`${base}/admin/users`);
    const open = page.getByRole('button', { name: 'Rates for Rates Person', exact: true });
    const dialog = page.getByRole('dialog', { name: 'Rates for Rates Person', exact: true });
    const billable = dialog.getByLabel('Default billable rate (EUR)', { exact: true });
    const cost = dialog.getByLabel('Cost rate (EUR)', { exact: true });
    await open.click();
    await expect(billable).toHaveValue('90.00');
    await expect(cost).toHaveValue('');
    if (evidence) await page.screenshot({ path: join(evidence, 'rates-admin-1440.png') });
    await billable.fill('95.505');
    await dialog.getByRole('button', { name: 'Save rates', exact: true }).click();
    await expect(dialog.getByRole('alert')).toContainText('at most two decimal places');
    assert.equal(stored(), '9000/-');
    await billable.fill('95.50');
    await cost.fill('40');
    await dialog.getByRole('button', { name: 'Save rates', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole('status').filter({ hasText: 'Rates saved.' })).toBeVisible();
    await expect(open).toBeFocused();
    assert.equal(stored(), '9550/4000');

    // An emptied field removes the rate; the other stays as it was.
    await open.click();
    await expect(billable).toHaveValue('95.50');
    await billable.fill('');
    await dialog.getByRole('button', { name: 'Save rates', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    assert.equal(stored(), '-/4000');

    await page.setViewportSize({ width: 390, height: 844 });
    await open.click();
    await expect(cost).toHaveValue('40.00');
    if (evidence) await page.screenshot({ path: join(evidence, 'rates-admin-390.png') });
    await page.keyboard.press('Escape');
    await expect(dialog).not.toBeVisible();

    // A read-only billable grant shows the billable rate as text and
    // withholds the cost rate entirely.
    restoreSeededStates = setAsideSeededStates(actor.org_id);
    sql(`BEGIN;
      UPDATE users SET billable_rate_cents=12000 WHERE id='${person}';
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('${state}','${actor.org_id}','${actor.id}',1,${array([...floor, 'people_read_managed', 'people_read_all','billable_rate_read_managed', 'billable_rate_read_all'])},false,'individual');
      COMMIT;`);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(`${base}/admin/users`);
    await page.getByRole('button', { name: 'Rates for Rates Person', exact: true }).click();
    await expect(dialog.locator('#person-billable-rate')).toHaveText('120.00');
    await expect(dialog.getByText('Cost rate', { exact: false })).toHaveCount(0);
    await expect(dialog.getByRole('button', { name: 'Save rates', exact: true })).toHaveCount(0);
    if (evidence) await page.screenshot({ path: join(evidence, 'rates-read-only-1440.png') });
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    assert.deepEqual(errors, []);
    console.log('PASS: person rates dialog edits, validates and clears exact rates, and withholds what a grant does not cover');
  } finally {
    await browser.close();
    sql(`BEGIN;
      DELETE FROM person_permission_states WHERE id='${state}'; DELETE FROM users WHERE id='${person}'; COMMIT;`);
    restoreSeededStates();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
