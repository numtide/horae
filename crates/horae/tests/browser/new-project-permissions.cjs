// Real sessions and mutations, only against the design runner's disposable DB.
// Project creation and its private fields follow the actor's stored grants.
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
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id, org_id, active, cost_rate_cents FROM users WHERE email = 'admin@example.com') u"));
assert.match(actor.id, /^[0-9a-f-]{36}$/);
assert.match(actor.org_id, /^[0-9a-f-]{36}$/);
assert.equal(actor.active, true);
// The seeded Administrator state, replaced per scenario and restored at the end.
const seededState = sql(`SELECT row_to_json(s) FROM person_permission_states s WHERE user_id = '${actor.id}'`);
assert.equal(JSON.parse(seededState).is_administrator, true);
const restoreState = () => sql(`BEGIN; DELETE FROM person_permission_states WHERE user_id = '${actor.id}';
  INSERT INTO person_permission_states SELECT * FROM json_populate_record(NULL::person_permission_states, $state$${seededState}$state$::json); COMMIT;`);
const setGrants = grants => sql(`UPDATE person_permission_states
  SET grants = ARRAY[${grants.map(grant => `'${grant}'`).join(',')}]::text[], is_administrator = false, source = 'individual', built_in_profile = NULL
  WHERE user_id = '${actor.id}'`);
// Grant sets with their prerequisites, after the built-in profiles they imitate.
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const creator = [...floor, 'project_read_managed', 'project_read_all', 'project_create_all', 'client_read_all', 'task_read_all'];
const projectManager = [...creator, 'project_write_managed'];
const executive = [...projectManager, 'project_write_all', 'billable_rate_read_managed', 'billable_rate_read_all', 'cost_rate_read_all'];
const other = '01960000-0000-7000-8000-000000000101';
const foreignOrg = '01960000-0000-7000-8000-000000000102';
const foreignClient = '01960000-0000-7000-8000-000000000103';
const foreignProject = '01960000-0000-7000-8000-000000000104';
const otherDraft = '01960000-0000-7000-8000-000000000105';
const ownDraft = '01960000-0000-7000-8000-000000000106';
const entry = '01960000-0000-7000-8000-000000000107';
const managedDraft = '01960000-0000-7000-8000-000000000108';
const privateNote = 'Permission fixture private note';
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
      INSERT INTO users (id, org_id, email, name) VALUES ('${other}', '${actor.org_id}', 'permission-owner@example.test', 'Other draft owner');
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
    await screen.getByLabel('Cost rate for Admin User (EUR/h)', { exact: true }).fill('47.25');
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
    // Without notes, costs or billable rates; a blank rate needs a mode that inherits.
    const publicForm = { ...creation.form, admin_notes: '', project_rate: '', rate_mode: 'person', team: creation.form.team.map(person => ({ ...person, cost_rate: '' })) };

    // The reports filters read list_projects; observe its route to replay it.
    await visit('/reports');
    await expect.poll(() => endpoints.has('list_projects')).toBe(true);
    await readsFinished();
    const optionsArgs = { search: { clients: { query: '', offset: 0 }, tasks: { query: '', offset: 0 }, people: { query: '', offset: 0 } } };
    // progress: reads the project; rates: billable rates; edit: edits it;
    // create: creates projects; notes/costs: writes private notes/cost rates;
    // costRead: reads cost rates; setRates: sets billable rates, fees, money
    // budgets and invoice defaults while creating.
    const cases = [
      { label: 'administrator', grants: null, visibility: 'managers', progress: true, rates: true, edit: true, create: true, notes: true, costs: true, costRead: true, setRates: true },
      { label: 'executive manager grants', grants: executive, visibility: 'managers', progress: true, rates: true, edit: true, create: true, notes: true, costs: false, costRead: true, setRates: false },
      { label: 'project manager grants', grants: projectManager, visibility: 'managers', progress: true, rates: false, edit: false, create: true, notes: false, costs: false, costRead: false, setRates: false },
      { label: 'visible assigned member', grants: floor, visibility: 'project_members', progress: true, rates: false, edit: false, create: false, notes: false, costs: false, costRead: false, setRates: false },
      { label: 'private assigned member', grants: floor, visibility: 'managers', progress: false, rates: false, edit: false, create: false, notes: false, costs: false, costRead: false, setRates: false },
    ];
    for (const scenario of cases) {
      await readsFinished();
      if (scenario.grants) setGrants(scenario.grants);
      else restoreState();
      sql(`UPDATE assignments SET role = 'freelancer' WHERE project_id = '${projectId}' AND user_id = '${actor.id}';
        UPDATE project_settings SET report_visibility = '${scenario.visibility}' WHERE project_id = '${projectId}'`);
      // Fixture visibility changes invalidate open editors. Reads and denied
      // mutations below must preserve the whole row, including that new revision.
      const originalProject = sql(`SELECT row_to_json(p)::text FROM projects p WHERE id = '${projectId}'`);
      await visit('/projects');
      // `endpoints` keeps earlier scenarios' reads, so it cannot show that this
      // page has hydrated; a click before hydration never opens the picker.
      await page.waitForLoadState('networkidle');
      await expect.poll(() => endpoints.has('list_project_budget_progress')).toBe(true);
      await readsFinished();
      // The rail reads the tracking catalog only when its timer picker opens.
      const catalog = page.waitForResponse(r => r.url().includes('/api/list_tracking_projects'));
      await page.locator('.sidebar-timer-wrap').getByRole('button', { name: 'Start timer', exact: true }).click();
      await catalog;
      await page.locator('.sidebar-timer-pop').getByRole('button', { name: 'Cancel', exact: true }).click();
      await readsFinished();
      await expect(page.locator('.page-actions').getByRole('link', { name: 'New project', exact: true })).toHaveCount(scenario.create ? 1 : 0);
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
      const details = await post('get_project_detail_view', { project_id: projectId });
      assert.equal(details.status(), scenario.progress ? 200 : 404);
      if (scenario.progress) {
        const body = await details.json();
        assert.deepEqual(body.requester, { org_id: actor.org_id, user_id: actor.id });
        assert.equal(body.can_edit, scenario.edit, scenario.label);
        assert.equal(body.can_change_status, scenario.edit, scenario.label);
        assert.equal(body.project.id, projectId);
        if (scenario.notes) assert.equal(body.project.admin_notes, privateNote);
        else absent(body.project, 'admin_notes');
        assert.deepEqual(body.team, [{ id: actor.id, name: 'Admin User' }]);
        assert.deepEqual(body.tasks, [{ id: taskId, name: 'Development' }]);
        // Detail labels never carry rates, even for an administrator.
        for (const person of body.team) absent(person, 'rate_cents');
        for (const task of body.tasks) absent(task, 'default_rate_cents');
      }
      assert.equal((await post('get_project_detail_view', { project_id: foreignProject })).status(), 404);
      await visit(`/projects/${projectId}`);
      const detailRegion = page.getByRole('region', { name: 'Project details', exact: true });
      if (scenario.progress) await expect(detailRegion).toContainText(name);
      else {
        await expect(page.getByRole('alert')).toContainText('Project details are unavailable');
        for (const label of ['Project details', 'Project team', 'Project tasks']) {
          await expect(page.getByRole('region', { name: label, exact: true })).toHaveCount(0);
        }
      }
      if (!scenario.notes) await expect(page.locator('body')).not.toContainText(privateNote);
      // Navigation may finish before hydration issues the permission request.
      const [optionsResponse] = await Promise.all([
        page.waitForResponse(response => /^\/api\/project_creation_options\d+$/.test(new URL(response.url()).pathname)),
        visit('/projects/new'),
      ]);
      assert.equal(optionsResponse.status(), scenario.create ? 200 : 403, scenario.label);
      if (scenario.create) {
        await expect(screen).toBeVisible();
        await expect(screen.getByLabel('Notes', { exact: true })).toHaveCount(scenario.notes ? 1 : 0);
        await expect(screen.getByLabel('Cost rate for Admin User (EUR/h)', { exact: true })).toHaveCount(scenario.costs ? 1 : 0);
        // The project rate is required in its mode, so the mode goes with the grant.
        // So does a fixed fee project's fee.
        for (const choice of [screen.getByRole('radio', { name: /^Project hourly rate/ }), screen.locator('input[name="np-project-type"][value="fixed_fee"]')]) {
          if (scenario.setRates) await expect(choice).toBeEnabled();
          else await expect(choice).toBeDisabled();
        }
        const options = await json('project_creation_options', optionsArgs);
        assert.equal(options.can_edit_private_notes, scenario.notes, scenario.label);
        assert.equal(options.can_edit_cost_rates, scenario.costs, scenario.label);
        assert.equal(options.can_edit_billable_settings, scenario.setRates, scenario.label);
        assert.equal(options.clients.some(client => client.id === foreignClient), false);
        if (!scenario.costRead) for (const person of options.people) absent(person, 'cost_rate_cents');
        // A saved draft is pruned to what the creator may still write.
        const draft = await json('load_project_draft');
        assert.equal(draft.id, ownDraft);
        assert.equal(draft.form.admin_notes, scenario.notes ? privateNote : '');
        assert.equal(draft.form.team[0].cost_rate, scenario.costs ? '47.25' : '');
        assert.equal(draft.form.project_rate, scenario.setRates ? '123.45' : '');
        if (!(scenario.notes && scenario.costs && scenario.setRates)) assert.equal((await post('save_project_draft', ownPayload)).status(), 403);
        if (!scenario.setRates) {
          for (const billable of [{ rate_mode: 'project', project_rate: '1' }, { invoice_defaults: { ...publicForm.invoice_defaults, po_number: 'PO-1' } }]) {
            const billableOnly = { ...ownPayload, form: { ...publicForm, ...billable } };
            assert.equal((await post('save_project_draft', billableOnly)).status(), 403, scenario.label);
          }
        }
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
      console.log(`PASS: ${scenario.label} real-session UI, draft ownership, progress scope and private-field redaction`);
    }

    await readsFinished();
    restoreState();
    const assignment = sql(`SELECT row_to_json(a)::text FROM assignments a WHERE project_id = '${projectId}' AND user_id = '${actor.id}'`);
    const cost = sql(`SELECT row_to_json(c)::text FROM project_member_costs c WHERE project_id = '${projectId}' AND user_id = '${actor.id}'`);
    setGrants(floor);
    sql(`DELETE FROM assignments WHERE project_id = '${projectId}' AND user_id = '${actor.id}'`);
    assert.equal((await json('list_projects', { client_id: null, include_inactive: true })).some(item => item.id === projectId), false);
    assert.equal((await post('get_project_detail_view', { project_id: projectId })).status(), 404);
    await visit(`/projects/${projectId}`);
    await expect(page.getByRole('alert')).toContainText('Project details are unavailable');
    await expect(page.getByRole('region', { name: 'Project team', exact: true })).toHaveCount(0);
    assert.ok((await json('list_tracking_projects')).some(item => item.id === projectId), 'Own history preserves identity, not project-wide access');
    sql(`UPDATE time_entries SET user_id = '${other}' WHERE id = '${entry}'`);
    assert.equal((await json('list_tracking_projects')).some(item => item.id === projectId), false);
    sql(`UPDATE time_entries SET user_id = '${actor.id}' WHERE id = '${entry}';
      INSERT INTO assignments SELECT * FROM json_populate_record(NULL::assignments, '${assignment.replaceAll("'", "''")}'::json);
      INSERT INTO project_member_costs SELECT * FROM json_populate_record(NULL::project_member_costs, '${cost.replaceAll("'", "''")}'::json)`);
    console.log('PASS: revoked assignment and outsider identities cannot grant project-wide access');

    restoreState();
    sql(`UPDATE users SET active = false WHERE id = '${actor.id}'`);
    for (const method of ['list_projects', 'get_project_detail_view', 'project_creation_options', 'load_project_draft', 'save_project_draft', 'finalize_project_draft']) {
      const response = await post(method, method === 'get_project_detail_view' ? { project_id: projectId } : method.includes('draft') && !method.startsWith('load') ? ownPayload : undefined);
      assert.ok([401, 403].includes(response.status()), `Inactive user: ${method} returned ${response.status()}`);
    }
    sql(`UPDATE users SET active = true WHERE id = '${actor.id}'`);
    setGrants(executive);
    // The creator marks themself as manager, which designates them like the editor would.
    const managerForm = { ...publicForm, name: 'Permission executive-created project', team: publicForm.team.map(person => ({ ...person, manager: true })) };
    const managerDraft = await json('save_project_draft', { ...ownPayload, form: managerForm });
    const managerProject = await json('finalize_project_draft', { ...ownPayload, expected_revision: managerDraft.revision, form: managerForm });
    assert.notEqual(managerProject, projectId);
    assert.equal(sql(`SELECT count(*) FROM project_member_costs WHERE project_id = '${managerProject}'`), '0');
    assert.equal(sql(`SELECT rate_cents IS NULL FROM projects WHERE id = '${managerProject}'`), 't');
    assert.equal(sql(`SELECT string_agg(manager_id::text, ',') FROM project_management_assignments WHERE project_id = '${managerProject}'`), actor.id);
    assert.equal(await json('load_project_draft'), null);
    // Without project_write_all the creator is designated even unmarked, so a
    // Project Manager can edit what they create.
    setGrants(projectManager);
    const managedForm = { ...publicForm, name: 'Permission project-manager-created project', team: publicForm.team.map(person => ({ ...person, manager: false })) };
    const managedPayload = { ...ownPayload, draft_id: managedDraft, expected_revision: 0, form: managedForm };
    const managedSaved = await json('save_project_draft', managedPayload);
    const managedProject = await json('finalize_project_draft', { ...managedPayload, expected_revision: managedSaved.revision });
    assert.equal(sql(`SELECT string_agg(manager_id::text, ',') FROM project_management_assignments WHERE project_id = '${managedProject}'`), actor.id);
    assert.equal((await (await post('get_project_detail_view', { project_id: managedProject })).json()).can_edit, true);
    const anonymous = await browser.newContext();
    try {
      for (const method of ['project_creation_options', 'load_project_draft', 'save_project_draft', 'finalize_project_draft']) {
        const response = await post(method, method.includes('draft') && !method.startsWith('load') ? ownPayload : undefined, anonymous.request);
        assert.equal(response.status(), 401, method);
      }
    } finally { await anonymous.close(); }
    assert.equal(sql(`SELECT payload::text FROM project_drafts WHERE id = '${otherDraft}'`), originalOtherDraft);
    assert.deepEqual(errors, []);
    console.log('PASS: inactive/anonymous requests are denied and project-creation grants create a real project without private overrides');
  } finally {
    await browser.close();
    sql(`UPDATE users SET active = true, cost_rate_cents = ${actor.cost_rate_cents ?? 'NULL'} WHERE id = '${actor.id}'`);
    restoreState();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
