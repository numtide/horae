// Canonical project reads use only the design runner's disposable database.
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
const id = n => `019f4100-0000-7000-8000-${String(n).padStart(12, '0')}`;
const project = id(1), client = id(2), teammate = id(3), task = id(4);
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const read = [...floor, 'project_read_managed'];
const edit = [...read, 'project_write_managed'];
const grantsSql = grants => `ARRAY[${grants.map(grant => `'${grant}'`).join(',')}]`;
const setGrants = grants => sql(`UPDATE person_permission_states SET grants=${grantsSql(grants)} WHERE user_id='${actor.id}'`);
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${org}'`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [], directories = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('request', request => {
    if (/\/api\/(list_users|list_clients|list_tasks)[^/]*$/.test(new URL(request.url()).pathname)) directories.push(request.url());
  });
  const assertHidden = async () => {
    await expect(page.getByRole('alert')).toContainText('Project details are unavailable');
    for (const label of ['Project details', 'Project team', 'Project tasks']) {
      await expect(page.getByRole('region', { name: label, exact: true })).toHaveCount(0);
    }
    await expect(page.getByRole('link', { name: 'Edit project', exact: true })).toHaveCount(0);
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    sql(`BEGIN;
      INSERT INTO clients (id,org_id,name,currency) VALUES ('${client}','${org}','Scoped read client','EUR');
      INSERT INTO users (id,org_id,email,name,active) VALUES ('${teammate}','${org}','private-teammate@example.test','Retained teammate',false);
      INSERT INTO projects (id,org_id,client_id,name,currency,rate_cents,budget_kind,budget_amount_cents)
        VALUES ('${project}','${org}','${client}','Scoped read project','EUR',12345,'amount',86753);
      INSERT INTO projects (id,org_id,client_id,name,currency)
        VALUES ('${id(9)}','${org}','${client}','Unrelated private project','EUR');
      INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,report_visibility)
        VALUES ('${id(10)}','${org}','${id(9)}','${actor.id}','project','managers');
      INSERT INTO project_private_settings (id,org_id,project_id,admin_notes) VALUES ('${id(5)}','${org}','${project}','Private launch note');
      INSERT INTO assignments (id,project_id,user_id,rate_cents) VALUES ('${id(6)}','${project}','${teammate}',76543);
      INSERT INTO tasks (id,org_id,name,default_rate_cents) VALUES ('${task}','${org}','Scoped read task',54321);
      INSERT INTO project_tasks (project_id,task_id,billable) VALUES ('${project}','${task}',true);
      INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES ('${id(7)}','${org}','${project}','${actor.id}');
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('${id(8)}','${org}','${actor.id}',1,${grantsSql(edit)},false,'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);

    const overviewResponse = page.waitForResponse(r => r.url().includes('/api/get_project_overview'));
    await page.goto(`${base}/projects`);
    const overview = await (await overviewResponse).json();
    // Seeded projects may independently share progress with this actor as a
    // teammate. Managed authority opens only this fixture, not the private one.
    const row = overview.projects.find(row => row.project.id === project);
    assert.ok(row);
    assert.deepEqual(overview.projects.filter(row => row.can_edit).map(row => row.project.id), [project]);
    assert.ok(!overview.projects.some(row => row.project.id === id(9)));
    assert.equal(row.client.name, 'Scoped read client');
    for (const { project } of overview.projects) {
      assert.ok(!Object.hasOwn(project, 'rate_cents'));
      assert.ok(!Object.hasOwn(project, 'budget_amount_cents'));
    }
    await expect(page.locator('.proj-row')).toHaveCount(overview.projects.length);
    await expect(page.getByRole('link', { name: 'New project', exact: true })).toHaveCount(0);
    const detailResponse = page.waitForResponse(r => r.url().includes('/api/get_project_detail_view'));
    await page.locator(`a[href="/projects/${project}"]`).click();
    const detail = await (await detailResponse).json();
    assert.deepEqual(detail.requester, { org_id: org, user_id: actor.id });
    assert.deepEqual(detail.team, [{ id: teammate, name: 'Retained teammate' }]);
    assert.deepEqual(detail.tasks, [{ id: task, name: 'Scoped read task' }]);
    assert.ok(!Object.hasOwn(detail.project, 'admin_notes'));
    for (const width of [1440, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await expect(page.getByRole('region', { name: 'Project team', exact: true })).toContainText('Retained teammate');
      await expect(page.getByRole('region', { name: 'Project tasks', exact: true })).toContainText('Scoped read task');
      await expect(page.getByRole('link', { name: 'Edit project', exact: true })).toBeVisible();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/project-read-${width}.png` });
    }
    await expect(page.getByRole('region', { name: 'Project fee balances', exact: true })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Assign User', exact: true })).toHaveCount(0);
    console.log('PASS: scoped project list/detail expose workflow labels without directory access, private notes or rates');

    setGrants(read);
    let release;
    const pending = new Promise(resolve => { release = resolve; });
    await page.route('**/api/get_project_detail_view*', async route => { await pending; await route.continue(); }, { times: 1 });
    try {
      await page.locator('#project-detail-refresh').click();
      await expect(page.getByText('Loading project details…', { exact: true })).toBeVisible();
      await expect(page.getByRole('region', { name: 'Project team', exact: true })).toHaveCount(0);
      await expect(page.getByRole('link', { name: 'Edit project', exact: true })).toHaveCount(0);
    } finally { release(); }
    await expect(page.getByRole('region', { name: 'Project team', exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Edit project', exact: true })).toHaveCount(0);

    sql(`DELETE FROM project_management_assignments WHERE id='${id(7)}'`);
    await page.locator('#project-detail-refresh').click();
    await assertHidden();
    sql(`INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES ('${id(7)}','${org}','${project}','${actor.id}')`);
    await page.locator('#project-detail-retry').click();
    await expect(page.getByRole('region', { name: 'Project team', exact: true })).toBeVisible();
    console.log('PASS: edit and managed-project revocation remove current UI authority without legacy Administrator fallback');

    sql(`UPDATE users SET active=false WHERE id='${actor.id}'`);
    await page.locator('#project-detail-refresh').click();
    await assertHidden();
    sql(`UPDATE users SET active=true WHERE id='${actor.id}'`);
    await page.locator('#project-detail-retry').click();
    await expect(page.getByRole('region', { name: 'Project team', exact: true })).toBeVisible();

    sql(`UPDATE organizations SET permission_policy_version=0 WHERE id='${org}'`);
    await page.locator('#project-detail-refresh').click();
    await assertHidden();
    sql(`UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'`);
    await page.locator('#project-detail-retry').click();
    await expect(page.getByRole('region', { name: 'Project team', exact: true })).toBeVisible();
    assert.deepEqual(directories, []);
    assert.deepEqual(errors, []);
    console.log('PASS: inactive-account and changed-policy responses clear retained detail; retry recovers only the original binding');
  } catch (error) {
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot() });
    throw error;
  } finally {
    await browser.close();
    sql(`BEGIN;
      UPDATE organizations SET permission_policy_version=0 WHERE id='${org}';
      UPDATE users SET active=true WHERE id='${actor.id}';
      DELETE FROM person_permission_states WHERE id='${id(8)}';
      DELETE FROM project_management_assignments WHERE project_id='${project}';
      DELETE FROM assignments WHERE project_id='${project}';
      DELETE FROM project_tasks WHERE project_id='${project}';
      DELETE FROM projects WHERE id IN ('${project}','${id(9)}');
      DELETE FROM clients WHERE id='${client}';
      DELETE FROM users WHERE id='${teammate}';
      DELETE FROM tasks WHERE id='${task}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
