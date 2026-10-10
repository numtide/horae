// Task identities and timer eligibility use only the runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id FROM users WHERE email='admin@example.com' AND active AND org_role='admin') u"));
const org = actor.org_id;
const id = n => `019f4200-0000-7000-8000-${String(n).padStart(12, '0')}`;
const project = id(1), client = id(2), historical = id(3), enabled = id(4), unlinked = id(5), timer = id(6);
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const array = values => `ARRAY[${values.map(value => `'${value}'`).join(',')}]`;
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${org}'`), '0');
assert.equal(sql(`SELECT count(*) FROM time_entries WHERE user_id='${actor.id}' AND is_running`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const identities = () => page.waitForResponse(r => r.url().includes('/api/list_tracking_tasks') && r.status() === 200);
  const rateFree = rows => {
    for (const row of rows) {
      assert.ok(!Object.hasOwn(row, 'default_rate_cents'));
      assert.ok(!Object.hasOwn(row, 'default_rate_currency'));
    }
  };
  const popover = page.locator('.sidebar-timer-pop');
  const openPicker = async () => {
    const response = identities();
    await page.locator('.sidebar-timer-wrap').getByRole('button', { name: 'Start timer', exact: true }).click();
    rateFree(await (await response).json());
    await popover.getByRole('combobox', { name: 'Project', exact: true }).selectOption(project);
    await expect(popover.getByRole('combobox', { name: 'Task', exact: true }).locator('option')).toHaveText(['Select task…', 'Enabled scoped task']);
  };
  try {
    sql(`BEGIN;
      INSERT INTO clients (id,org_id,name,currency) VALUES ('${client}','${org}','Task reader client','EUR');
      INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ('${project}','${org}','${client}','Task reader project','EUR');
      INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,report_visibility)
        VALUES ('${id(7)}','${org}','${project}','${actor.id}','project','managers');
      INSERT INTO assignments (id,project_id,user_id) VALUES ('${id(8)}','${project}','${actor.id}');
      INSERT INTO tasks (id,org_id,name,active,default_rate_cents,default_rate_currency) VALUES
        ('${historical}','${org}','Archived timer task',false,45678,'EUR'),
        ('${enabled}','${org}','Enabled scoped task',true,56789,'EUR'),
        ('${unlinked}','${org}','Unlinked catalog task',true,67890,'EUR');
      INSERT INTO project_tasks (project_id,task_id,billable) VALUES ('${project}','${historical}',true),('${project}','${enabled}',true);
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,is_running,started_at)
        VALUES ('${timer}','${org}','${actor.id}','${project}','${historical}',CURRENT_DATE,0,true,true,now());
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('${id(9)}','${org}','${actor.id}',1,${array(floor)},false,'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);
    await page.goto(`${base}/auth/login`);
    const response = identities();
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    const rows = await (await response).json();
    assert.ok(rows.some(row => row.id === historical));
    assert.ok(rows.some(row => row.id === enabled));
    assert.ok(!rows.some(row => row.id === unlinked));
    rateFree(rows);
    await expect(page.locator('.sidebar-timer-proj')).toHaveText('Task reader project · Archived timer task');
    const deniedCatalog = await context.request.get(`${base}/harvest/v2/tasks`);
    assert.equal(deniedCatalog.status(), 200);
    assert.equal((await deniedCatalog.json()).total_entries, 0);
    console.log('PASS: archived own timer labels survive without granting catalog or global-rate access');

    await page.getByRole('button', { name: 'Stop timer', exact: true }).click();
    await expect(page.locator('.sidebar-timer-live')).toHaveCount(0);
    assert.equal(sql(`SELECT is_running FROM time_entries WHERE id='${timer}'`), 'f');
    await openPicker();
    await popover.getByRole('combobox', { name: 'Task', exact: true }).selectOption(enabled);
    await popover.getByRole('button', { name: 'Start timer', exact: true }).click();
    await expect(page.locator('.sidebar-timer-proj')).toHaveText('Task reader project · Enabled scoped task');
    assert.equal(sql(`SELECT task_id FROM time_entries WHERE user_id='${actor.id}' AND is_running`), enabled);
    await page.getByRole('button', { name: 'Stop timer', exact: true }).click();
    await expect(page.locator('.sidebar-timer-live')).toHaveCount(0);

    sql(`BEGIN; UPDATE organizations SET access_revision=access_revision+1 WHERE id='${org}';
      UPDATE person_permission_states SET grants=${array([...floor, 'task_read_all', 'billable_rate_read_managed', 'billable_rate_read_all'])}, revision=revision+1
        WHERE user_id='${actor.id}' AND org_id='${org}'; COMMIT;`);
    const expanded = identities();
    await page.reload();
    const catalogIdentities = await (await expanded).json();
    assert.ok(catalogIdentities.some(row => row.id === unlinked));
    rateFree(catalogIdentities);
    const catalog = await context.request.get(`${base}/harvest/v2/tasks/${unlinked}`);
    assert.equal(catalog.status(), 200);
    assert.equal((await catalog.json()).default_hourly_rate, 678.9);
    await openPicker();
    await expect(page.locator('.sidebar-resize')).toHaveCSS('pointer-events', 'none');
    await popover.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(popover).toHaveCount(0);
    await expect(page.locator('.sidebar-resize')).toHaveCSS('pointer-events', 'auto');
    const handle = await page.locator('.sidebar-resize').boundingBox();
    await page.mouse.move(handle.x + handle.width / 2, 100);
    await page.mouse.down();
    await page.mouse.move(320, 100);
    await page.mouse.up();
    await expect(page.locator('.app-sidebar')).toHaveCSS('width', '320px');
    await page.locator('.sidebar-footer').click();
    await expect(page.locator('.sidebar-menu')).toBeVisible();
    await expect(page.locator('.sidebar-resize')).toHaveCSS('pointer-events', 'none');
    await page.locator('.sidebar-footer').click();
    await expect(page.locator('.sidebar-resize')).toHaveCSS('pointer-events', 'auto');
    assert.deepEqual(errors, []);
    console.log('PASS: catalog/rate grants never enable unlinked or archived tasks for a new timer; valid start/stop persists');
    console.log('PASS: rail popovers accept clicks and release the resize handle after closing');
  } catch (error) {
    console.error(error);
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot({ timeout: 3000 }).catch(failure => failure.message) });
    throw error;
  } finally {
    await browser.close();
    sql(`BEGIN; UPDATE organizations SET permission_policy_version=0 WHERE id='${org}';
      DELETE FROM person_permission_states WHERE id='${id(9)}';
      DELETE FROM time_entries WHERE project_id='${project}';
      DELETE FROM assignments WHERE project_id='${project}';
      DELETE FROM project_tasks WHERE project_id='${project}';
      DELETE FROM projects WHERE id='${project}';
      DELETE FROM clients WHERE id='${client}';
      DELETE FROM tasks WHERE id IN ('${historical}','${enabled}','${unlinked}'); COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
