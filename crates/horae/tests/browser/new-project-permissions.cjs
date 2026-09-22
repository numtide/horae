// Real sessions and mutations, only against the design runner's disposable DB.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id, org_id, org_role, active, cost_rate_cents FROM users WHERE email = 'admin@example.com') u"));
assert.match(actor.id, /^[0-9a-f-]{36}$/);
assert.match(actor.org_id, /^[0-9a-f-]{36}$/);
assert.equal(actor.org_role, 'admin');
assert.equal(actor.active, true);
const other = '01960000-0000-7000-8000-000000000101';
const foreignOrg = '01960000-0000-7000-8000-000000000102';
const foreignClient = '01960000-0000-7000-8000-000000000103';
const foreignProject = '01960000-0000-7000-8000-000000000104';
const otherDraft = '01960000-0000-7000-8000-000000000105';
const ownDraft = '01960000-0000-7000-8000-000000000106';
const entry = '01960000-0000-7000-8000-000000000107';
const privateNote = 'Permission fixture administrator-only note';
const name = 'Permission fixture project';

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const pending = new Set(), endpoints = new Map(), errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('request', request => {
    const match = new URL(request.url()).pathname.match(/^\/api\/([a-z_]+)\d+$/);
    if (!match) return;
    pending.add(request);
    endpoints.set(match[1], { url: request.url(), data: request.postDataJSON(), headers: { 'content-type': request.headers()['content-type'] } });
  });
  page.on('requestfinished', request => pending.delete(request));
  page.on('requestfailed', request => pending.delete(request));
  const readsFinished = () => expect.poll(() => pending.size).toBe(0);
  const visit = async path => { await readsFinished(); await page.goto(`${base}${path}`); };
  const post = (method, data, request = context.request) => {
    const endpoint = endpoints.get(method);
    assert.ok(endpoint, `Observe the actual ${method} route before replaying it`);
    return request.post(endpoint.url, { headers: endpoint.headers, data: JSON.stringify(data ?? endpoint.data) });
  };
  const json = async (method, data) => {
    const response = await post(method, data);
    assert.equal(response.status(), 200, `${method}: ${await response.text()}`);
    return response.json();
  };
  const absent = (value, field) => assert.equal(Object.hasOwn(value, field), false, `${field} must be omitted, not merely hidden by the UI`);
  let projectId;
  try {
    sql(`UPDATE users SET cost_rate_cents = 4321 WHERE id = '${actor.id}';
      INSERT INTO users (id, org_id, email, name, org_role) VALUES ('${other}', '${actor.org_id}', 'permission-owner@example.test', 'Other draft owner', 'admin');
      INSERT INTO organizations (id, name) VALUES ('${foreignOrg}', 'Permission foreign organization');
      INSERT INTO clients (id, org_id, name, currency) VALUES ('${foreignClient}', '${foreignOrg}', 'Permission foreign client', 'EUR');
      INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ('${foreignProject}', '${foreignOrg}', '${foreignClient}', 'Permission foreign project', 'EUR')`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await visit('/projects/new');
    const screen = page.locator('.np-page');
    await screen.getByRole('button', { name: 'Client', exact: true }).click();
    await page.getByRole('dialog', { name: 'Choose Client', exact: true }).getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await screen.getByLabel('Project name', { exact: true }).fill(name);
    await screen.getByLabel('Notes', { exact: true }).fill(privateNote);
    await screen.getByRole('radio', { name: /^Project hourly rate/ }).check();
    await screen.locator('#np-project-rate').fill('123.45');
    await screen.locator('#np-budget-mode').click();
    await page.getByRole('listbox', { name: 'Choose Budget', exact: true }).getByRole('option', { name: 'Total project hours', exact: true }).click();
    await screen.locator('#np-budget-value').fill('10');
    await screen.getByLabel('Tags', { exact: true }).fill('permission-matrix');
    await screen.getByLabel('Tags', { exact: true }).press('Enter');
    await screen.getByRole('button', { name: 'Development', exact: true }).click();
    await screen.locator('#np-add-person').click();
    await page.getByRole('dialog', { name: 'Choose teammate', exact: true }).getByRole('option', { name: 'Admin User', exact: true }).click();
    await screen.getByLabel('Cost rate for Admin User (EUR/h) · admins only', { exact: true }).fill('47.25');
    await expect(screen.locator('header').getByRole('status')).toContainText('Draft saved at');
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await page.waitForURL(/\/projects\/[0-9a-f-]{36}$/);
    projectId = page.url().split('/').at(-1);
    const creation = structuredClone(endpoints.get('finalize_project_draft').data);
    assert.equal(creation.form.admin_notes, privateNote);
    await expect(page.getByRole('region', { name: 'Project details', exact: true })).toContainText(privateNote);
    await readsFinished();
    const taskId = creation.form.tasks[0].source.task_id;
    assert.match(taskId, /^[0-9a-f-]{36}$/);
    sql(`INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
      VALUES ('${entry}', '${actor.org_id}', '${actor.id}', '${projectId}', '${taskId}', CURRENT_DATE, 60, true);
      INSERT INTO project_drafts (id, org_id, creator_id, payload)
      SELECT '${otherDraft}', org_id, '${other}', payload FROM project_drafts WHERE id = '${creation.draft_id}'`);
    const originalOtherDraft = sql(`SELECT payload::text FROM project_drafts WHERE id = '${otherDraft}'`);
    const saved = await json('save_project_draft', { ...creation, draft_id: ownDraft, expected_revision: 0 });
    const ownPayload = { ...creation, draft_id: ownDraft, expected_revision: saved.revision };
    const publicForm = { ...creation.form, admin_notes: '', team: creation.form.team.map(person => ({ ...person, cost_rate: '' })) };

    await visit('/reports');
    await expect.poll(() => endpoints.has('report_time')).toBe(true);
    await readsFinished();
    const today = sql('SELECT CURRENT_DATE::text');
    const reportArgs = { from: today, to: today, group_by: 'project', client_id: null, project_id: projectId, user_id: null, tag_id: null };
    const optionsArgs = { search: { clients: { query: '', offset: 0 }, tasks: { query: '', offset: 0 }, people: { query: '', offset: 0 } } };
    const cases = [
      { label: 'organization admin', role: 'admin', assignment: 'freelancer', visibility: 'managers', progress: true, rates: true, private: true },
      { label: 'organization manager', role: 'manager', assignment: 'freelancer', visibility: 'managers', progress: true, rates: true, private: false },
      { label: 'project lead', role: 'member', assignment: 'lead', visibility: 'managers', progress: true, rates: false, private: false },
      { label: 'project admin', role: 'member', assignment: 'admin', visibility: 'managers', progress: true, rates: false, private: false },
      { label: 'visible assigned member', role: 'member', assignment: 'freelancer', visibility: 'project_members', progress: true, rates: false, private: false },
      { label: 'private assigned member', role: 'member', assignment: 'freelancer', visibility: 'managers', progress: false, rates: false, private: false },
    ];
    for (const scenario of cases) {
      await readsFinished();
      sql(`UPDATE users SET org_role = '${scenario.role}' WHERE id = '${actor.id}';
        UPDATE assignments SET role = '${scenario.assignment}' WHERE project_id = '${projectId}' AND user_id = '${actor.id}';
        UPDATE project_settings SET report_visibility = '${scenario.visibility}' WHERE project_id = '${projectId}'`);
      // Fixture role/visibility changes invalidate open editors. Reads and denied
      // mutations below must preserve the whole row, including that new revision.
      const originalProject = sql(`SELECT row_to_json(p)::text FROM projects p WHERE id = '${projectId}'`);
      await visit('/projects');
      await expect.poll(() => endpoints.has('list_project_budget_progress')).toBe(true);
      await readsFinished();
      await expect(page.getByRole('link', { name: 'New project', exact: true })).toHaveCount(scenario.rates ? 1 : 0);
      const projects = await json('list_projects', { client_id: null, include_inactive: true });
      const project = projects.find(item => item.id === projectId);
      assert.equal(!!project, scenario.progress, scenario.label);
      assert.equal(projects.some(item => item.id === foreignProject), false);
      if (project) {
        if (scenario.rates) assert.equal(project.rate_cents, 12345);
        else absent(project, 'rate_cents');
      }
      await expect(page.locator('.proj-row').filter({ hasText: name })).toHaveCount(scenario.progress ? 1 : 0);
      for (const method of ['list_project_spend', 'list_project_budget_progress', 'list_project_tags']) {
        assert.equal((await json(method)).some(item => item.project_id === projectId), scenario.progress, `${scenario.label}: ${method}`);
      }
      const tracking = (await json('list_tracking_projects')).find(item => item.id === projectId);
      assert.ok(tracking, 'Own tracking remains available regardless of report visibility');
      for (const field of ['rate_cents', 'budget_minutes', 'budget_amount_cents']) absent(tracking, field);
      const details = await post('get_project_details', { project_id: projectId });
      assert.equal(details.status(), scenario.progress ? 200 : 404);
      if (scenario.progress) {
        const body = await details.json();
        if (scenario.private) assert.equal(body.admin_notes, privateNote);
        else absent(body, 'admin_notes');
      }
      assert.equal((await post('get_project_details', { project_id: foreignProject })).status(), 404);
      const team = await json('list_assignments', { project_id: projectId });
      const tasks = await json('list_project_tasks', { project_id: projectId });
      assert.equal(team.some(person => person.user_id === actor.id), true);
      assert.equal(tasks.some(task => task.id === taskId), true);
      if (!scenario.rates) {
        for (const person of team) absent(person, 'rate_cents');
        for (const task of tasks) absent(task, 'default_rate_cents');
      }
      const exported = await context.request.get(`${base}/api/projects/export/csv?scope=active`);
      assert.equal(exported.status(), 200);
      assert.equal((await exported.text()).includes(name), scenario.progress);
      const report = await post('report_time', reportArgs);
      assert.equal(report.status(), scenario.rates ? 200 : 403);
      if (scenario.rates) {
        const [row] = await report.json();
        assert.equal(row.billable_cents, 12345);
        if (scenario.private) assert.equal(row.cost_cents, 4725);
        else absent(row, 'cost_cents');
      }
      await visit(`/projects/${projectId}`);
      const detailRegion = page.getByRole('region', { name: 'Project details', exact: true });
      if (scenario.progress) await expect(detailRegion).toContainText(name);
      else await expect(detailRegion.getByRole('alert')).toContainText('Could not load project details');
      if (!scenario.private) await expect(detailRegion).not.toContainText(privateNote);
      await visit('/projects/new');
      if (scenario.rates) {
        await expect(screen).toBeVisible();
        await expect(screen.getByLabel('Notes', { exact: true })).toHaveCount(scenario.private ? 1 : 0);
        await expect(screen.getByLabel('Cost rate for Admin User (EUR/h) · admins only', { exact: true })).toHaveCount(scenario.private ? 1 : 0);
        const options = await json('project_creation_options', optionsArgs);
        assert.equal(options.can_edit_private_settings, scenario.private);
        assert.equal(options.clients.some(client => client.id === foreignClient), false);
        if (!scenario.private) for (const person of options.people) absent(person, 'cost_rate_cents');
        const draft = await json('load_project_draft');
        assert.equal(draft.id, ownDraft);
        assert.equal(draft.form.admin_notes, scenario.private ? privateNote : '');
        assert.equal(draft.form.team[0].cost_rate, scenario.private ? '47.25' : '');
        if (!scenario.private) assert.equal((await post('save_project_draft', ownPayload)).status(), 403);
        const foreignWrite = { ...ownPayload, draft_id: otherDraft, expected_revision: 1, form: publicForm };
        assert.equal((await post('save_project_draft', foreignWrite)).status(), 404);
        assert.equal((await post('finalize_project_draft', foreignWrite)).status(), 404);
      } else {
        await expect(page.getByRole('alert')).toContainText('Could not load project settings');
        await expect(screen).toHaveCount(0);
        for (const [method, data] of [['project_creation_options', optionsArgs], ['load_project_draft', {}], ['save_project_draft', ownPayload], ['finalize_project_draft', ownPayload]]) {
          assert.equal((await post(method, data)).status(), 403, `${scenario.label}: ${method}`);
        }
      }
      assert.equal(sql(`SELECT payload::text FROM project_drafts WHERE id = '${otherDraft}'`), originalOtherDraft);
      assert.equal(sql(`SELECT row_to_json(p)::text FROM projects p WHERE id = '${projectId}'`), originalProject);
      console.log(`PASS: ${scenario.label} real-session UI, draft ownership, progress/export scope and financial redaction`);
    }

    await readsFinished();
    const assignment = sql(`SELECT row_to_json(a)::text FROM assignments a WHERE project_id = '${projectId}' AND user_id = '${actor.id}'`);
    const cost = sql(`SELECT row_to_json(c)::text FROM project_member_costs c WHERE project_id = '${projectId}' AND user_id = '${actor.id}'`);
    sql(`DELETE FROM assignments WHERE project_id = '${projectId}' AND user_id = '${actor.id}'`);
    assert.equal((await json('list_projects', { client_id: null, include_inactive: true })).some(item => item.id === projectId), false);
    assert.equal((await post('get_project_details', { project_id: projectId })).status(), 404);
    assert.deepEqual(await json('list_assignments', { project_id: projectId }), []);
    assert.ok((await json('list_tracking_projects')).some(item => item.id === projectId), 'Own history preserves identity, not project-wide access');
    sql(`UPDATE time_entries SET user_id = '${other}' WHERE id = '${entry}'`);
    assert.equal((await json('list_tracking_projects')).some(item => item.id === projectId), false);
    sql(`UPDATE time_entries SET user_id = '${actor.id}' WHERE id = '${entry}';
      INSERT INTO assignments SELECT * FROM json_populate_record(NULL::assignments, '${assignment.replaceAll("'", "''")}'::json);
      INSERT INTO project_member_costs SELECT * FROM json_populate_record(NULL::project_member_costs, '${cost.replaceAll("'", "''")}'::json)`);
    console.log('PASS: revoked assignment and outsider identities cannot grant project-wide access');

    sql(`UPDATE users SET org_role = 'admin', active = false WHERE id = '${actor.id}'`);
    for (const method of ['list_projects', 'get_project_details', 'project_creation_options', 'load_project_draft', 'save_project_draft', 'finalize_project_draft']) {
      const response = await post(method, method === 'get_project_details' ? { project_id: projectId } : method.includes('draft') && !method.startsWith('load') ? ownPayload : undefined);
      assert.ok([401, 403].includes(response.status()), `Inactive user: ${method} returned ${response.status()}`);
    }
    sql(`UPDATE users SET org_role = 'manager', active = true WHERE id = '${actor.id}'`);
    const managerDraft = await json('save_project_draft', { ...ownPayload, form: { ...publicForm, name: 'Permission manager-created project' } });
    const managerProject = await json('finalize_project_draft', { ...ownPayload, expected_revision: managerDraft.revision, form: { ...publicForm, name: 'Permission manager-created project' } });
    assert.notEqual(managerProject, projectId);
    assert.equal(sql(`SELECT count(*) FROM project_member_costs WHERE project_id = '${managerProject}'`), '0');
    assert.equal(await json('load_project_draft'), null);
    const anonymous = await browser.newContext();
    try {
      for (const method of ['project_creation_options', 'load_project_draft', 'save_project_draft', 'finalize_project_draft']) {
        const response = await post(method, method.includes('draft') && !method.startsWith('load') ? ownPayload : undefined, anonymous.request);
        assert.equal(response.status(), 401, method);
      }
    } finally { await anonymous.close(); }
    assert.equal(sql(`SELECT payload::text FROM project_drafts WHERE id = '${otherDraft}'`), originalOtherDraft);
    assert.deepEqual(errors, []);
    console.log('PASS: inactive/anonymous requests are denied and a manager creates a real project without private overrides');
  } finally {
    await browser.close();
    sql(`UPDATE users SET org_role = '${actor.org_role}', active = true, cost_rate_cents = ${actor.cost_rate_cents ?? 'NULL'} WHERE id = '${actor.id}'`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
