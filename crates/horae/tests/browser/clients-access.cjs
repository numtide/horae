// Real browser sessions and direct server calls against the disposable runner only.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const id = suffix => `01970000-0000-7000-8000-${suffix.toString().padStart(12, '0')}`;
const foreignOrg = id(401), foreignClient = id(402), visibleProject = id(403), hiddenProject = id(404);
const name = 'Access scope client';

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  let actor;
  const endpoints = new Map(), pending = new Set(), errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    const match = new URL(request.url()).pathname.match(/^\/api\/([a-z_]+)\d+$/);
    if (!match) return;
    pending.add(request);
    endpoints.set(match[1], { url: request.url(), data: request.postDataJSON(), headers: { 'content-type': request.headers()['content-type'] } });
  });
  page.on('requestfinished', request => pending.delete(request));
  page.on('requestfailed', request => pending.delete(request));
  const settled = () => expect.poll(() => pending.size).toBe(0);
  const visit = async path => { await settled(); await page.goto(`${base}${path}`); };
  const post = async (method, data, session = context.request) => {
    const endpoint = endpoints.get(method);
    assert.ok(endpoint, `Observe ${method} in the real UI before replaying it`);
    return session.post(endpoint.url, { headers: endpoint.headers, data: JSON.stringify(data ?? endpoint.data) });
  };
  const json = async (method, data) => {
    const response = await post(method, data);
    assert.equal(response.status(), 200, `${method}: ${await response.text()}`);
    return response.json();
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await visit('/clients');
    await expect.poll(() => endpoints.has('get_me')).toBe(true);
    // Dev login can select another admin created by a preceding suite.
    actor = await json('get_me', {});
    assert.match(actor.id, /^[0-9a-f-]{36}$/);
    assert.match(actor.org_id, /^[0-9a-f-]{36}$/);
    assert.equal(actor.org_role, 'admin');
    assert.deepEqual(Object.keys(actor).sort(), ['email', 'id', 'name', 'org_id', 'org_role']);
    assert.equal(sql(`SELECT active FROM users WHERE id='${actor.id}' AND org_id='${actor.org_id}'`), 't');
    await page.getByRole('button', { name: 'New client', exact: true }).first().click();
    const dialog = page.getByRole('dialog');
    await dialog.getByLabel('Client name', { exact: true }).fill(name);
    await dialog.getByLabel('Default rate', { exact: true }).fill('123.45');
    await dialog.getByLabel('Currency', { exact: true }).selectOption('EUR');
    await dialog.getByRole('button', { name: 'Create client', exact: true }).click();
    await expect(dialog).toBeHidden();
    const client = sql(`SELECT id FROM clients WHERE name = '${name}'`);
    assert.match(client, /^[0-9a-f-]{36}$/);
    sql(`INSERT INTO organizations (id, name) VALUES ('${foreignOrg}', 'Access foreign organization');
      INSERT INTO clients (id, org_id, name, currency, default_rate_cents) VALUES ('${foreignClient}', '${foreignOrg}', 'Access foreign secret', 'GBP', 9876543);
      INSERT INTO projects (id, org_id, client_id, name, currency) VALUES
        ('${visibleProject}', '${actor.org_id}', '${client}', 'Access visible project', 'USD'),
        ('${hiddenProject}', '${actor.org_id}', '${client}', 'Access hidden project', 'CHF');
      INSERT INTO assignments (id, project_id, user_id) VALUES
        ('${id(405)}', '${visibleProject}', '${actor.id}'), ('${id(406)}', '${hiddenProject}', '${actor.id}');
      INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, report_visibility) VALUES
        ('${id(407)}', '${actor.org_id}', '${visibleProject}', '${actor.id}', 'person', 'project_members'),
        ('${id(408)}', '${actor.org_id}', '${hiddenProject}', '${actor.id}', 'person', 'managers');
      INSERT INTO invoices (id, org_id, client_id, number, issued_on, due_on, currency) VALUES
        ('${id(409)}', '${actor.org_id}', '${client}', 'ACCESS-OWN', CURRENT_DATE, CURRENT_DATE, 'EUR'),
        ('${id(410)}', '${foreignOrg}', '${foreignClient}', 'ACCESS-FOREIGN', CURRENT_DATE, CURRENT_DATE, 'GBP')`);
    await visit(`/clients/${client}`);
    await page.getByRole('button', { name: 'Edit client', exact: true }).click();
    await expect(dialog.getByLabel('Default rate', { exact: true })).toHaveValue('123.45');
    await dialog.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(dialog).toBeHidden();
    await settled();
    const createPayload = structuredClone(endpoints.get('create_client_profile').data);
    const editPayload = structuredClone(endpoints.get('update_client_profile').data);
    const baseline = sql(`SELECT row_to_json(c) FROM clients c WHERE id = '${client}'`);
    for (const patch of [{ default_rate: '-1' }, { profile: { ...createPayload.profile, currency: 'XYZ' } }]) {
      assert.equal((await post('create_client_profile', { ...createPayload, ...patch })).status(), 400);
    }
    const reads = [['get_client_details', { client_id: client }], ['list_client_summaries', {}], ['list_client_invoices', { client_id: client }]];
    for (const scenario of [
      { role: 'admin', assignment: 'freelancer', count: 2, billing: true },
      { role: 'manager', assignment: 'freelancer', count: 2, billing: true },
      { role: 'member', assignment: 'lead', count: 2, billing: false },
      { role: 'member', assignment: 'freelancer', count: 1, billing: false },
    ]) {
      await settled();
      sql(`UPDATE users SET org_role = '${scenario.role}' WHERE id = '${actor.id}';
        UPDATE assignments SET role = '${scenario.assignment}' WHERE id IN ('${id(405)}', '${id(406)}')`);
      await visit('/clients');
      await page.getByRole('searchbox', { name: 'Search clients by name' }).fill(name);
      const row = page.getByRole('row').filter({ has: page.getByRole('link', { name, exact: true }) });
      await expect(row.getByLabel(`${scenario.count} active of ${scenario.count} visible projects`)).toBeVisible();
      await expect(page.getByRole('button', { name: 'New client', exact: true })).toHaveCount(scenario.billing ? 1 : 0);
      const summaries = await json('list_client_summaries', {});
      assert.equal(summaries.some(item => item.client.id === foreignClient), false);
      const summary = summaries.find(item => item.client.id === client);
      assert.equal(summary.total_projects, scenario.count);
      assert.deepEqual(summary.project_currencies.sort(), scenario.count === 2 ? ['CHF', 'USD'] : ['USD']);
      assert.equal(Object.hasOwn(summary.client, 'default_rate_cents'), false);
      if (scenario.count === 1) await expect(row).not.toContainText('CHF');
      await page.getByRole('link', { name, exact: true }).click();
      await expect(page.getByRole('heading', { name, exact: true })).toBeVisible();
      await expect(page.getByRole('link', { name: 'Access visible project', exact: true })).toBeVisible();
      await expect(page.getByRole('link', { name: 'Access hidden project', exact: true })).toHaveCount(scenario.count === 2 ? 1 : 0);
      await expect(page.getByRole('button', { name: 'Edit client', exact: true })).toHaveCount(scenario.billing ? 1 : 0);
      await expect(page.getByRole('link', { name: 'New invoice', exact: true })).toHaveCount(scenario.billing ? 1 : 0);
      const detail = await json('get_client_details', { client_id: client });
      assert.equal(Object.hasOwn(detail, 'billing'), scenario.billing);
      const projectRows = await json('list_projects', { client_id: client, include_inactive: true });
      assert.equal(projectRows.length, scenario.count);
      assert.equal(projectRows.some(project => project.id === hiddenProject), scenario.count === 2);
      if (!scenario.billing) for (const project of projectRows) assert.equal(Object.hasOwn(project, 'rate_cents'), false);
      if (scenario.billing) {
        assert.equal(detail.billing.default_rate_cents, 12345);
        await expect(page.getByRole('link', { name: 'ACCESS-OWN', exact: true })).toBeVisible();
        assert.deepEqual((await json('list_client_invoices', { client_id: client })).map(item => item.id), [id(409)]);
        assert.equal((await post('list_client_invoices', { client_id: foreignClient })).status(), 404);
      } else {
        await expect(page.getByRole('region', { name: 'Invoices', exact: true })).toHaveCount(0);
        assert.equal((await post('list_client_invoices', { client_id: client })).status(), 403);
        assert.equal((await post('create_client_profile', createPayload)).status(), 403);
        assert.equal((await post('update_client_profile', editPayload)).status(), 403);
        await visit(`/projects/new/client/${client}`);
        await expect(page.getByRole('alert')).toContainText('Could not load project settings');
        await expect(page.locator('.np-page')).toHaveCount(0);
        await visit(`/invoices/new/client/${client}`);
        await expect(page.getByRole('status')).toContainText('Invoices require manager access');
        await expect(page.locator('#inv-client')).toHaveCount(0);
      }
      const foreign = await post('get_client_details', { client_id: foreignClient });
      const missing = await post('get_client_details', { client_id: id(499) });
      assert.equal(foreign.status(), 404);
      assert.equal(missing.status(), 404);
      assert.equal(await foreign.text(), await missing.text());
      await visit(`/clients/${foreignClient}`);
      await expect(page.getByRole('alert')).toContainText('Could not load this client');
      await expect(page.locator('body')).not.toContainText('Access foreign secret');
      assert.equal(sql(`SELECT row_to_json(c) FROM clients c WHERE id = '${client}'`), baseline);
      console.log(`PASS: clients ${scenario.role}/${scenario.assignment} real-session scope, counts, currencies, billing and negative direct access`);
    }
    await settled();
    sql(`UPDATE users SET active = false WHERE id = '${actor.id}'`);
    for (const [method, data] of [...reads, ['create_client_profile', createPayload], ['update_client_profile', editPayload]]) {
      assert.ok([401, 403].includes((await post(method, data)).status()), `inactive ${method}`);
    }
    sql(`UPDATE users SET active = true, org_role = 'admin' WHERE id = '${actor.id}'`);
    const foreignEdit = { ...editPayload, client_id: foreignClient };
    assert.equal((await post('update_client_profile', foreignEdit)).status(), 404);
    const anonymous = await browser.newContext();
    try {
      for (const [method, data] of [...reads, ['create_client_profile', createPayload], ['update_client_profile', editPayload]]) {
        assert.equal((await post(method, data, anonymous.request)).status(), 401, `anonymous ${method}`);
      }
    } finally { await anonymous.close(); }
    assert.equal(sql(`SELECT row_to_json(c) FROM clients c WHERE id = '${client}'`), baseline);
    assert.deepEqual(errors, []);
    console.log('PASS: clients inactive/anonymous access, foreign mutation and direct validation boundaries');
  } finally {
    await browser.close();
    if (actor) sql(`UPDATE users SET org_role = '${actor.org_role}', active = true WHERE id = '${actor.id}'`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
