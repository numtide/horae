// Adding, archiving and restoring people on the People page, by stored grant.
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
const email = `people-management-${Date.now()}@example.com`;
const state = '019f4500-0000-7000-8000-000000000001';
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const array = values => `ARRAY[${values.map(value => `'${value}'`).join(',')}]`;
const policy = sql(`SELECT permission_policy_version FROM organizations WHERE id='${actor.org_id}'`);
let restoreSeededStates = () => {};

(async () => {
  const browser = await playwright.chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const row = name => page.locator('tbody tr').filter({ hasText: name });
  try {
    // The People page answers from stored permissions only.
    sql(`UPDATE organizations SET permission_policy_version=1 WHERE id='${actor.org_id}'`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/admin/users`);

    // An administrator adds a person with a profile; Administrator is not offered.
    await page.locator('#people-add').click();
    const form = page.locator('#people-add-form');
    await expect(form.locator('#people-add-profile option[value="administrator"]')).toHaveCount(0);
    await form.getByLabel('Email', { exact: true }).fill(email);
    await form.getByLabel('Name', { exact: true }).fill('Managed Person');
    await form.locator('#people-add-profile').selectOption('people_admin');
    if (evidence) await page.screenshot({ path: join(evidence, 'people-add-1440.png') });
    await form.getByRole('button', { name: 'Add person', exact: true }).click();
    await expect(page.getByRole('status').filter({ hasText: 'Managed Person was added.' })).toBeVisible();
    await expect(row('Managed Person')).toBeVisible();
    assert.equal(sql(`SELECT p.built_in_profile || '/' || u.active FROM users u
      JOIN person_permission_states p ON p.user_id = u.id WHERE u.email='${email}'`), 'people_admin/true');

    // Archiving moves the person out of the active list; restoring brings them back.
    await page.getByRole('button', { name: 'Archive Managed Person', exact: true }).click();
    await expect(row('Managed Person')).toHaveCount(0);
    assert.equal(sql(`SELECT active FROM users WHERE email='${email}'`), 'f');
    await page.locator('#people-activity').selectOption('archived');
    await page.getByRole('button', { name: 'Restore Managed Person', exact: true }).click();
    await expect(row('Managed Person')).toHaveCount(0);
    assert.equal(sql(`SELECT active FROM users WHERE email='${email}'`), 't');
    await page.locator('#people-activity').selectOption('active');

    // The last administrator cannot archive themselves.
    const self = sql(`SELECT name FROM users WHERE id='${actor.id}'`);
    await page.getByRole('button', { name: `Archive ${self}`, exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('at least one active administrator');
    assert.equal(sql(`SELECT active FROM users WHERE id='${actor.id}'`), 't');
    console.log('PASS: an administrator adds people with a profile, archives and restores them, and keeps an administrator');

    // A People Admin adds Members only and cannot edit permissions. The actor
    // takes the grants the new People Admin was given.
    const peopleAdmin = sql(`SELECT p.grants FROM person_permission_states p
      JOIN users u ON u.id = p.user_id WHERE u.email='${email}'`);
    restoreSeededStates = setAsideSeededStates(actor.org_id);
    sql(`INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
      VALUES ('${state}','${actor.org_id}','${actor.id}',1,'${peopleAdmin}',false,'individual')`);
    await page.reload();
    await expect(row('Managed Person')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Edit permissions for Managed Person', exact: true })).toHaveCount(0);
    await page.locator('#people-add').click();
    await expect(page.locator('#people-add-profile')).toHaveCount(0);
    await expect(page.locator('#people-add-email')).toBeVisible();
    if (evidence) await page.screenshot({ path: join(evidence, 'people-add-people-admin-1440.png') });
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();

    // Without people management there is nothing to add or archive.
    sql(`UPDATE person_permission_states SET grants=${array([...floor, 'people_read_managed', 'people_read_all'])} WHERE id='${state}'`);
    await page.reload();
    await expect(row('Managed Person')).toBeVisible();
    await expect(page.locator('#people-add')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Archive Managed Person', exact: true })).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log('PASS: People Admins add Members only, and the page offers no management without the grant');
  } finally {
    await browser.close();
    // The set-aside states include the added person's, so they go back first.
    sql(`DELETE FROM person_permission_states WHERE id='${state}'`);
    restoreSeededStates();
    sql(`BEGIN; UPDATE organizations SET permission_policy_version=${policy} WHERE id='${actor.org_id}';
      DELETE FROM person_permission_states WHERE user_id IN (SELECT id FROM users WHERE email='${email}');
      DELETE FROM users WHERE email='${email}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
