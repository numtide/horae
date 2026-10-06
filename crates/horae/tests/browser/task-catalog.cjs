// This suite changes only the database created by run-design-checks.sh.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const { join } = require('node:path');
const evidence = process.env.HORAE_TASK_CATALOG_EVIDENCE;
if (evidence) mkdirSync(evidence, { recursive: true });
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id FROM users WHERE email='admin@example.com' AND active) u"));
const task = '019f4300-0000-7000-8000-000000000001';
const state = '019f4300-0000-7000-8000-000000000002';
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const grants = ['task_read_all', 'task_write_all', 'billable_rate_read_managed', 'billable_rate_read_all', 'billable_rate_write_managed', 'billable_rate_write_all'];
const array = values => `ARRAY[${values.map(value => `'${value}'`).join(',')}]`;
const authorize = extra => sql(`BEGIN;
  UPDATE organizations SET access_revision=access_revision+1 WHERE id='${actor.org_id}';
  UPDATE person_permission_states SET grants=${array([...floor, ...extra])}, revision=revision+1 WHERE id='${state}'; COMMIT;`);
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${actor.org_id}'`), '0');
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${actor.org_id}'`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  try {
    sql(`BEGIN;
      INSERT INTO tasks (id,org_id,name,billable_default,default_rate_cents,default_rate_currency)
        VALUES ('${task}','${actor.org_id}','Catalog browser task',true,8000,'USD');
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('${state}','${actor.org_id}','${actor.id}',1,${array([...floor, ...grants])},false,'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${actor.org_id}'; COMMIT;`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/admin/tasks`);
    const row = page.getByRole('row').filter({ hasText: 'Catalog browser task' });
    await expect(row).toContainText('USD 80.00');
    await expect(row.getByRole('cell', { name: 'USD 80.00', exact: true })).toHaveCSS('white-space', 'nowrap');
    await expect(page.locator('.adm-nav')).not.toContainText('People');
    for (const width of [1440, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.evaluate(theme => document.documentElement.setAttribute('data-theme', theme), width === 1440 ? 'dark' : 'light');
      await row.getByRole('button', { name: 'Edit Catalog browser task', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Edit task', exact: true });
      await expect(dialog).toBeVisible();
      await expect(dialog).toContainText('Current default: USD 80.00');
      if (evidence) await page.screenshot({ path: join(evidence, `task-editor-${width}.png`), fullPage: true });
      await page.keyboard.press('Escape');
      await expect(dialog).not.toBeVisible();
      await expect(row.getByRole('button')).toBeFocused();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
      if (evidence) {
        await page.locator('.table-container').evaluate(node => { node.scrollLeft = 0; });
        await page.screenshot({ path: join(evidence, `task-catalog-${width}.png`), fullPage: true });
      }
    }
    await page.setViewportSize({ width: 1440, height: 900 });
    await row.getByRole('button').click();
    const dialog = page.getByRole('dialog', { name: 'Edit task', exact: true });
    await dialog.getByLabel('Default hourly rate', { exact: true }).selectOption('set');
    await dialog.getByLabel('Hourly rate (EUR)', { exact: true }).fill('0');
    await dialog.getByRole('button', { name: 'Save task', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(row).toContainText('EUR 0.00');
    assert.equal(sql(`SELECT default_rate_cents || ':' || default_rate_currency FROM tasks WHERE id='${task}'`), '0:EUR');

    authorize(['task_read_all', 'task_write_all']);
    await page.getByRole('button', { name: 'Refresh tasks', exact: true }).click();
    await expect(page.getByRole('columnheader', { name: 'Default hourly rate', exact: true })).toHaveCount(0);
    await row.getByRole('button').click();
    await expect(dialog.locator('#task-rate-action')).toHaveCount(0);
    await dialog.getByLabel('Task name', { exact: true }).fill('Catalog browser renamed');
    await dialog.getByRole('button', { name: 'Save task', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    assert.equal(sql(`SELECT default_rate_cents || ':' || default_rate_currency FROM tasks WHERE id='${task}'`), '0:EUR');
    const renamed = page.getByRole('row').filter({ hasText: 'Catalog browser renamed' });
    await renamed.getByRole('button').click();
    await dialog.getByRole('button', { name: 'Archive task…', exact: true }).click();
    await expect(dialog).toContainText('every project');
    await dialog.getByRole('button', { name: 'Confirm archive', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(renamed).toHaveCount(0);
    await page.getByLabel('Show', { exact: true }).selectOption('archived');
    await renamed.getByRole('button').click();
    await dialog.getByRole('button', { name: 'Restore task…', exact: true }).click();
    await expect(dialog).toContainText('restore it separately');
    await dialog.getByRole('button', { name: 'Confirm restore', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await page.getByLabel('Show', { exact: true }).selectOption('active');
    await renamed.getByRole('button').click();
    authorize(['task_read_all']);
    await dialog.getByRole('button', { name: 'Save task', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(page.getByText('You have read-only access to tasks.', { exact: true })).toBeVisible();
    await expect(renamed.getByRole('button')).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log('PASS: task catalog desktop/mobile keyboard, exact rates, hidden-rate preservation, archive/restore and revoked editing');
  } finally {
    await browser.close();
    sql(`BEGIN; UPDATE organizations SET permission_policy_version=0 WHERE id='${actor.org_id}';
      DELETE FROM person_permission_states WHERE id='${state}'; DELETE FROM tasks WHERE id='${task}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
