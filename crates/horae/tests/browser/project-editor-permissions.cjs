// Canonical editor acceptance uses only run-design-checks.sh's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const { mkdirSync } = require('node:fs');
const { join } = require('node:path');
const assert = require('node:assert/strict');
const evidence = process.env.HORAE_PROJECT_TASK_EVIDENCE;
if (evidence) mkdirSync(evidence, { recursive: true });
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id FROM users WHERE email='admin@example.com' AND active AND org_role='admin') u"));
const org = actor.org_id;
const id = n => `019f4000-0000-7000-8000-${String(n).padStart(12, '0')}`;
const project = id(1), client = id(2), archived = id(3), outside = id(4), active = id(12);
const task = id(20), archivedTask = id(21), globalArchivedTask = id(22);
const taskState = () => sql(`SELECT jsonb_agg(jsonb_build_array(pt.task_id,pt.billable,pt.rate_cents,s.budget_minutes,s.budget_cents,s.restricted) ORDER BY pt.task_id)
  FROM project_tasks pt JOIN project_task_settings s USING(project_id,task_id) WHERE pt.project_id='${project}'`);
const taskActivity = taskId => sql(`SELECT active FROM project_tasks WHERE project_id='${project}' AND task_id='${taskId}'`);
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const edit = [...floor, 'project_read_managed', 'project_read_all', 'project_write_managed', 'project_write_all'];
const readMoney = ['billable_rate_read_managed', 'billable_rate_read_all', 'cost_rate_read_all'];
const writeMoney = [...readMoney, 'billable_rate_write_managed', 'billable_rate_write_all', 'cost_rate_write_all'];
const grantsSql = grants => `ARRAY[${grants.map(grant => `'${grant}'`).join(',')}]`;
const setGrants = grants => sql(`UPDATE person_permission_states SET grants=${grantsSql(grants)} WHERE user_id='${actor.id}'`);
const history = () => sql(`SELECT json_build_array(
  (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM time_entries t),
  (SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM invoices i),
  (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l))`);
const protectedState = () => sql(`SELECT json_build_array(
  (SELECT jsonb_build_array(rate_cents,budget_minutes,budget_amount_cents) FROM projects WHERE id='${project}'),
  (SELECT po_number FROM project_settings WHERE project_id='${project}'),
  (SELECT admin_notes FROM project_private_settings WHERE project_id='${project}'),
  (SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM project_member_costs c WHERE project_id='${project}'))`);
const leaves = value => value && typeof value === 'object' ? Object.values(value).flatMap(leaves) : [value];
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${org}'`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const open = async () => {
    await page.goto(`${base}/projects/${project}/edit`);
    await expect(page.locator('#np-name')).toBeVisible();
  };
  const save = async () => {
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${project}`);
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    const originalHistory = history();
    sql(`BEGIN;
      INSERT INTO clients (id,org_id,name,currency) VALUES ('${client}','${org}','Permission editor client','EUR');
      INSERT INTO users (id,org_id,email,name,active) VALUES
        ('${archived}','${org}','editor-archived@example.test','Archived teammate',false),
        ('${outside}','${org}','editor-outside@example.test','Outside archived manager',false),
        ('${active}','${org}','editor-active@example.test','Active teammate',true);
      INSERT INTO projects (id,org_id,client_id,name,currency,rate_cents,budget_kind,budget_minutes,budget_amount_cents)
        VALUES ('${project}','${org}','${client}','Canonical editor fixture','EUR',12345,'hours',600,86753);
      INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,po_number)
        VALUES ('${id(5)}','${org}','${project}','${actor.id}','project','Private purchase order');
      INSERT INTO project_private_settings (id,org_id,project_id,admin_notes)
        VALUES ('${id(6)}','${org}','${project}','Private project note');
      INSERT INTO assignments (id,project_id,user_id) VALUES
        ('${id(7)}','${project}','${archived}'), ('${id(13)}','${project}','${active}');
      INSERT INTO tasks (id,org_id,name,active) VALUES
        ('${task}','${org}','Retained project task',true),
        ('${archivedTask}','${org}','Archived project task',true),
        ('${globalArchivedTask}','${org}','Globally archived task',false);
      INSERT INTO project_tasks (project_id,task_id,active,billable,rate_cents) VALUES
        ('${project}','${task}',true,true,23456),
        ('${project}','${archivedTask}',false,true,34567),
        ('${project}','${globalArchivedTask}',false,true,45678);
      INSERT INTO project_task_settings (id,org_id,project_id,task_id,restricted,budget_minutes) VALUES
        ('${id(26)}','${org}','${project}','${task}',true,120),
        ('${id(27)}','${org}','${project}','${archivedTask}',false,240),
        ('${id(28)}','${org}','${project}','${globalArchivedTask}',false,360);
      INSERT INTO project_task_members (id,org_id,project_id,task_id,user_id)
        VALUES ('${id(29)}','${org}','${project}','${task}','${active}');
      INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents)
        VALUES ('${id(8)}','${org}','${project}','${archived}',56789),
          ('${id(14)}','${org}','${project}','${active}',67890);
      INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES
        ('${id(9)}','${org}','${project}','${archived}'), ('${id(10)}','${org}','${project}','${outside}');
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('${id(11)}','${org}','${actor.id}',1,${grantsSql(edit)},false,'individual');
      UPDATE users SET org_role='member' WHERE id='${actor.id}';
      UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);
    const protectedBefore = protectedState();
    const editorResponse = page.waitForResponse(response => response.url().includes('/api/load_project_editor'));
    await open();
    const disclosed = await (await editorResponse).json();
    for (const secret of [12345, 56789, 67890, 86753, '123.45', '567.89', '678.90', '867.53', 'Private purchase order', 'Private project note']) {
      assert.ok(!leaves(disclosed).includes(secret), `withheld editor response disclosed ${secret}`);
    }
    for (const field of ['#np-project-rate', '#np-notes', '#np-po-number', `#np-cost-rate-${archived}`, `#np-cost-rate-${active}`]) {
      await expect(page.locator(field)).toHaveCount(0);
    }
    await expect(page.locator('#np-budget-value')).toBeEnabled();
    for (const width of [1440, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await expect(page.locator(`#np-person-manager-${outside}`)).toBeEnabled();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
    }
    await page.locator(`#np-person-manager-${archived}`).focus();
    await page.keyboard.press('Space');
    await expect(page.locator(`#np-person-manager-${archived}`)).toHaveAttribute('aria-checked', 'false');
    await expect(page.locator(`#np-person-remove-${archived}`)).toBeDisabled();
    await page.locator(`#np-person-manager-${outside}`).click();
    await page.locator('#np-name').fill('Ordinary authorized edit');
    await save();
    assert.equal(sql(`SELECT count(*) FROM project_management_assignments WHERE project_id='${project}'`), '0');
    assert.equal(sql(`SELECT count(*) FROM assignments WHERE project_id='${project}' AND user_id='${archived}'`), '1');
    assert.equal(protectedState(), protectedBefore);
    console.log('PASS: keyboard manager removal preserves archived membership and withheld money/notes at desktop and narrow widths');

    const tasksBefore = taskState();
    assert.equal(taskActivity(archivedTask), 'f', 'unrelated save must not restore a project task');
    await open();
    const taskSection = page.getByRole('region', { name: 'Tasks', exact: true });
    await expect(taskSection.getByRole('button', { name: 'Restore task Archived project task', exact: true })).toBeEnabled();
    await expect(taskSection.getByRole('button', { name: 'Restore task Globally archived task', exact: true })).toBeDisabled();
    await expect(taskSection.getByText('Restore in the task catalog first.', { exact: true })).toBeVisible();
    const archiveTask = () => taskSection.getByRole('button', { name: 'Archive task Retained project task', exact: true });
    await archiveTask().focus();
    await page.keyboard.press('Enter');
    await expect(taskSection.getByRole('button', { name: 'Restore task Retained project task', exact: true })).toBeFocused();
    await expect(page.locator('[data-project-edit-state]')).toHaveAttribute('data-project-edit-state', 'dirty');
    await expect(taskSection.getByRole('button', { name: 'Access for Retained project task: Restricted (1)', exact: true })).toBeDisabled();
    assert.equal(taskActivity(task), 't', 'staging must not write before Save');
    await taskSection.getByRole('button', { name: 'Restore task Retained project task', exact: true }).click();
    await expect(page.locator('[data-project-edit-state]')).toHaveAttribute('data-project-edit-state', 'clean');
    await taskSection.getByRole('button', { name: 'Development', exact: true }).click();
    await taskSection.getByRole('button', { name: 'Remove task Development', exact: true }).click();
    await expect(page.locator('[data-project-edit-state]')).toHaveAttribute('data-project-edit-state', 'clean');
    await archiveTask().click();
    for (const width of [1440, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.evaluate(theme => document.documentElement.setAttribute('data-theme', theme), width === 1440 ? 'dark' : 'light');
      await taskSection.scrollIntoViewIfNeeded();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
      if (evidence) await taskSection.screenshot({ path: join(evidence, `project-task-lifecycle-${width}.png`) });
    }
    await page.locator('#np-cancel').click();
    await expect(page.getByRole('heading', { name: 'Discard changes?', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Keep editing', exact: true }).click();
    sql(`INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,state,is_running,started_at)
      VALUES ('${id(30)}','${org}','${active}','${project}','${task}',CURRENT_DATE,0,true,'open',true,now())`);
    await page.locator('#np-save').click();
    await expect(page.locator('#np-form-error-message')).toHaveText('A running timer prevents archiving this project task');
    await expect(taskSection.getByRole('button', { name: 'Restore task Retained project task', exact: true })).toBeEnabled();
    await expect(page.locator('#np-save')).toBeEnabled();
    assert.equal(taskActivity(task), 't');
    assert.equal(taskState(), tasksBefore);
    sql(`DELETE FROM time_entries WHERE id='${id(30)}'`);
    await save();
    assert.equal(taskActivity(task), 'f');
    assert.equal(taskState(), tasksBefore);
    assert.equal(sql(`SELECT user_id FROM project_task_members WHERE project_id='${project}' AND task_id='${task}'`), active);
    await open();
    await taskSection.getByRole('button', { name: 'Restore task Archived project task', exact: true }).click();
    await save();
    assert.equal(taskActivity(archivedTask), 't');
    assert.equal(taskActivity(task), 'f');
    assert.equal(taskState(), tasksBefore);
    console.log('PASS: staged project task archive/restore, undo, cancellation and running-timer rejection preserve hidden configuration and restrictions');

    setGrants([...edit, ...readMoney]);
    await open();
    await expect(page.locator('#np-project-rate')).toHaveValue('123.45');
    await expect(page.locator('#np-project-rate')).toBeDisabled();
    await expect(page.locator(`#np-cost-rate-${archived}`)).toHaveValue('567.89');
    await expect(page.locator(`#np-cost-rate-${archived}`)).toBeDisabled();
    await expect(page.locator(`#np-cost-rate-${active}`)).toHaveValue('678.90');
    await expect(page.locator(`#np-cost-rate-${active}`)).toBeDisabled();
    await expect(page.locator('#np-notes')).toHaveCount(0);
    await expect(page.locator(`#np-person-manager-${archived}`)).toBeDisabled();
    await page.locator('#np-name').fill('Read-only financial edit');
    await save();
    assert.equal(protectedState(), protectedBefore);
    console.log('PASS: financial read grants display disabled values and ordinary saves preserve stored fields');

    setGrants([...edit, ...writeMoney]);
    await open();
    await taskSection.getByRole('button', { name: 'None', exact: true }).click();
    await expect(taskSection.getByRole('checkbox', { name: 'Retained project task is billable', exact: true })).toHaveAttribute('aria-checked', 'true');
    await expect(taskSection.getByRole('checkbox', { name: 'Archived project task is billable', exact: true })).toHaveAttribute('aria-checked', 'false');
    await taskSection.getByRole('button', { name: 'All', exact: true }).click();
    await expect(page.locator('#np-project-rate')).toBeEnabled();
    await page.locator('#np-project-rate').fill('0');
    await expect(page.locator(`#np-cost-rate-${active}`)).toBeEnabled();
    await page.locator(`#np-cost-rate-${active}`).fill('');
    await taskSection.getByRole('button', { name: 'Restore task Retained project task', exact: true }).click();
    const requests = [];
    await page.route('**/api/save_project_editor*', async route => {
      requests.push(route.request().postData());
      const response = await route.fetch();
      assert.equal(response.status(), 200, await response.text());
      await route.abort('failed');
    }, { times: 1 });
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.locator('#np-retry')).toBeVisible();
    assert.equal(sql(`SELECT rate_cents FROM projects WHERE id='${project}'`), '0');
    const committedRevision = sql(`SELECT edit_revision FROM projects WHERE id='${project}'`);
    await page.route('**/api/save_project_editor*', route => {
      requests.push(route.request().postData());
      return route.continue();
    }, { times: 1 });
    await page.locator('#np-retry').click();
    await expect(page).toHaveURL(`${base}/projects/${project}`);
    assert.equal(requests.length, 2);
    assert.equal(requests[0], requests[1]);
    assert.equal(taskActivity(task), 't');
    assert.equal(sql(`SELECT edit_revision FROM projects WHERE id='${project}'`), committedRevision);
    const zeroExpected = JSON.parse(protectedBefore);
    zeroExpected[0][0] = 0;
    zeroExpected[3] = zeroExpected[3].filter(row => row.user_id !== active);
    assert.deepEqual(JSON.parse(protectedState()), zeroExpected);
    console.log('PASS: explicit zero, cost reset and lost-acknowledgement replay preserve exact original intent');

    await open();
    await page.locator('#np-name').fill('Discard on access loss');
    await archiveTask().click();
    setGrants(floor);
    await page.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page.locator('#np-editor-reload')).toBeVisible();
    await expect(page.locator('#np-name')).toHaveCount(0);
    await expect(page.locator('#np-retry')).toHaveCount(0);
    setGrants(edit);
    await page.locator('#np-editor-reload').click();
    await expect(page.locator('#np-name')).toHaveValue('Read-only financial edit');
    await expect(page.locator('#np-project-rate')).toHaveCount(0);
    assert.equal(taskActivity(task), 't', 'revoked save must not archive the task');
    console.log('PASS: current-authority revocation discards old editor state; explicit reload restores a fresh authorized form');

    await page.locator('#np-budget-value').fill('11');
    await save();
    const hoursExpected = structuredClone(zeroExpected);
    hoursExpected[0][1] = 660;
    assert.deepEqual(JSON.parse(protectedState()), hoursExpected);
    assert.equal(history(), originalHistory);
    assert.deepEqual(errors, []);
    console.log('PASS: an explicit hours-only budget edit preserves inactive hidden money and recorded history');
  } finally {
    await browser.close();
    sql(`BEGIN;
      UPDATE organizations SET permission_policy_version=0 WHERE id='${org}';
      UPDATE users SET org_role='admin' WHERE id='${actor.id}';
      DELETE FROM person_permission_states WHERE id='${id(11)}';
      DELETE FROM time_entries WHERE id='${id(30)}';
      DELETE FROM project_management_assignments WHERE project_id='${project}';
      DELETE FROM assignments WHERE project_id='${project}';
      DELETE FROM project_tasks WHERE project_id='${project}';
      DELETE FROM projects WHERE id='${project}';
      DELETE FROM tasks WHERE id IN ('${task}','${archivedTask}','${globalArchivedTask}');
      DELETE FROM clients WHERE id='${client}';
      DELETE FROM users WHERE id IN ('${archived}','${outside}','${active}'); COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
