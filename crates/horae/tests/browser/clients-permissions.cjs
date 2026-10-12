// Canonical Clients reads and refresh fences use only the design runner's disposable database.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const expect = baseExpect.configure({ timeout: 15000 });
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const output = process.env.HORAE_TEST_SCREENSHOT_DIR;
if (output) require('node:fs').mkdirSync(output, { recursive: true });
const id = n => `019f4200-0000-7000-8000-${String(n).padStart(12, '0')}`;
const unset = id(1), zero = id(2), project = id(3), state = id(4), assignment = id(5), stranger = id(6);
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const catalog = [...floor, 'client_read_all'];
const grantsSql = grants => `ARRAY[${grants.map(grant => `'${grant}'`).join(',')}]`;
const setGrants = grants => sql(`UPDATE person_permission_states SET grants=${grantsSql(grants)} WHERE id='${state}'`);
let actor, org, owned = false;
const setPolicy = version => sql(`UPDATE organizations SET permission_policy_version=${version} WHERE id='${org}'`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const dialog = page.getByRole('dialog');
  const sidebarClients = page.locator('.app-sidebar').getByRole('link', { name: 'Clients', exact: true });
  const billing = page.getByRole('region', { name: 'Billing', exact: true });
  // Focus restoration runs on the next animation frame after a reload settles.
  const activeId = () => page.evaluate(() => new Promise(resolve =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve(document.activeElement?.id || '')))));
  const capture = async name => {
    if (output) await page.screenshot({ path: `${output}/clients-permissions-${name}.png` });
  };
  const loadList = async () => {
    const response = page.waitForResponse(r => r.url().includes('/api/client_overview'));
    await page.goto(`${base}/clients`);
    return response;
  };
  const success = async response => {
    assert.equal(response.status(), 200, `${response.url()}: ${await response.text()}`);
    return response.json();
  };
  const loadDetail = async client => {
    const response = page.waitForResponse(r => r.url().includes('/api/client_detail_view'));
    await page.goto(`${base}/clients/${client}`);
    return response;
  };
  const editTaxId = async (opener, value) => {
    await opener.click();
    await expect(dialog.getByLabel('Client name', { exact: true })).toBeFocused();
    await dialog.getByLabel('Tax ID (optional)', { exact: true }).fill(value);
    await dialog.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(dialog).toBeHidden();
  };
  try {
    const login = await context.request.post(`${base}/auth/dev-login`, { maxRedirects: 0 });
    assert.equal(login.status(), 303);
    // Dev login can select another admin created by a preceding suite.
    const me = page.waitForResponse(r => r.url().includes('/api/get_me') && r.status() === 200);
    await page.goto(`${base}/clients`);
    actor = await (await me).json();
    org = actor.org_id;
    assert.equal(actor.org_role, 'admin');
    // Only this suite's rows are created and removed; the policy is restored to its observed value.
    assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');
    assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE user_id='${actor.id}'`), '0');
    assert.equal(sql(`SELECT count(*) FROM clients WHERE id IN ('${unset}','${zero}')`), '0');
    owned = true;
    sql(`BEGIN;
      INSERT INTO clients (id,org_id,name,currency) VALUES ('${unset}','${org}','Permission unset client','EUR');
      INSERT INTO clients (id,org_id,name,currency,default_rate_cents) VALUES ('${zero}','${org}','Permission zero client','EUR',0);
      INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ('${project}','${org}','${unset}','Permission managed project','EUR');
      INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES ('${assignment}','${org}','${project}','${actor.id}');
      COMMIT;`);

    // Policy zero: a save's reload that fails, is rebound or changes policy
    // consumes the opener refocus instead of moving focus later.
    await loadList();
    await page.getByRole('searchbox', { name: 'Search clients by name' }).fill('Permission unset client');
    const rowEdit = page.locator(`#client-edit-${unset}`);
    await page.route('**/api/client_overview*', route => route.abort('failed'), { times: 1 });
    await editTaxId(rowEdit, 'PERM-1');
    await expect(page.getByRole('alert')).toContainText('Could not load clients');
    await expect(page.locator('#clients-heading')).toBeFocused();
    await page.getByRole('button', { name: 'Retry clients', exact: true }).click();
    await expect(rowEdit).toBeVisible();
    assert.notEqual(await activeId(), `client-edit-${unset}`);

    let refresh;
    await page.route('**/api/client_overview*', async route => {
      refresh = route.request().postDataJSON();
      const response = await route.fetch();
      const body = await response.json();
      await route.fulfill({ response, json: { ...body, requester: { ...body.requester, user_id: stranger } } });
    }, { times: 1 });
    await editTaxId(rowEdit, 'PERM-2');
    await expect(page.getByRole('alert')).toContainText('Could not load clients');
    assert.deepEqual(refresh.expected_requester, { org_id: org, user_id: actor.id }, 'the reload asks the server to check the bound account');
    await expect(page.getByRole('button', { name: 'New client', exact: true })).toHaveCount(0);
    await page.getByRole('button', { name: 'Retry clients', exact: true }).click();
    await expect(rowEdit).toBeVisible();
    assert.notEqual(await activeId(), `client-edit-${unset}`);
    await editTaxId(rowEdit, 'PERM-3');
    await expect(rowEdit).toBeFocused();
    console.log('PASS: list refocus is one-shot across failed and rebound reloads; a successful edit returns to its row');

    await loadDetail(unset);
    const edit = page.locator('#client-edit');
    await page.route('**/api/client_detail_view*', async route => {
      sql(`BEGIN;
        INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
          VALUES ('${state}','${org}','${actor.id}',1,${grantsSql(catalog)},false,'individual');
        UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);
      await route.continue();
    }, { times: 1 });
    await editTaxId(edit, 'PERM-4');
    await expect(page.getByRole('alert')).toContainText('Could not load this client');
    await expect(page.getByRole('heading', { name: 'Permission unset client', exact: true })).toHaveCount(0);
    await expect(edit).toHaveCount(0);
    await page.getByRole('button', { name: 'Retry client', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('Could not load this client');
    setPolicy(0);
    await page.getByRole('button', { name: 'Retry client', exact: true }).click();
    await expect(edit).toBeVisible();
    assert.notEqual(await activeId(), 'client-edit');
    await editTaxId(edit, 'PERM-5');
    await expect(edit).toBeFocused();
    console.log('PASS: a policy change during a detail reload is rejected until the original policy returns');

    // Policy one: catalog reads without management, even for a legacy Administrator.
    setPolicy(1);
    let overview = await success(await loadList());
    assert.deepEqual([overview.canonical_permissions, overview.can_read, overview.can_manage], [true, true, false]);
    assert.ok([unset, zero].every(client => overview.clients.some(row => row.client.id === client)));
    await expect(page.getByRole('link', { name: 'Permission zero client', exact: true })).toBeVisible();
    await expect(sidebarClients).toBeVisible();
    for (const control of ['New client', 'Edit', 'Deactivate', 'Activate']) {
      await expect(page.getByRole('button', { name: control, exact: true })).toHaveCount(0);
    }
    await expect(page.getByRole('link', { name: 'Import', exact: true })).toHaveCount(0);
    await capture('read-only-list');
    for (const client of [unset, zero]) {
      const view = await success(await loadDetail(client));
      assert.deepEqual([view.canonical_permissions, view.can_manage, Object.hasOwn(view.details, 'billing')], [true, false, false]);
      await expect(billing).toBeVisible();
      await expect(billing).not.toContainText('Default rate');
    }
    for (const control of ['Edit client', 'New project', 'New invoice']) {
      await expect(page.getByRole(control === 'Edit client' ? 'button' : 'link', { name: control, exact: true })).toHaveCount(0);
    }
    console.log('PASS: ClientReadAll reads the catalog without client management or default rates');

    setGrants([...catalog, 'project_read_managed', 'billable_rate_read_managed']);
    const managed = await success(await loadDetail(unset));
    assert.equal(Object.hasOwn(managed.details, 'billing'), false);
    await expect(billing).not.toContainText('Default rate');

    setGrants([...catalog, 'billable_rate_read_managed', 'billable_rate_read_all']);
    for (const [client, rate, text] of [[unset, null, 'Not set'], [zero, 0, 'EUR 0.00 / h']]) {
      const view = await success(await loadDetail(client));
      assert.deepEqual(view.details.billing, { default_rate_cents: rate });
      await expect(billing.locator('dt:text-is("Default rate") + dd')).toHaveText(text);
      await expect(page.getByRole('button', { name: 'Edit client', exact: true })).toHaveCount(0);
    }
    await capture('global-rate-detail');
    console.log('PASS: only global billable-rate read reveals the client default, keeping unset distinct from zero');

    setGrants([...floor, 'project_read_managed', 'project_read_all', 'invoice_read_managed', 'invoice_read_all',
      'billable_rate_read_managed', 'billable_rate_read_all', 'report_profitability_read']);
    overview = await success(await loadList());
    assert.deepEqual([overview.can_read, overview.can_manage, overview.clients], [false, false, []]);
    await expect(page.getByRole('heading', { name: 'Client access is unavailable', exact: true })).toBeVisible();
    await expect(page.getByText('No clients yet', { exact: true })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'New client', exact: true })).toHaveCount(0);
    await expect(sidebarClients).toHaveCount(0);
    await capture('denied-list');
    const denied = await loadDetail(zero);
    assert.equal(denied.status(), 404);
    await expect(page.getByRole('alert')).toContainText('Could not load this client');
    await expect(page.getByRole('heading', { name: 'Permission zero client', exact: true })).toHaveCount(0);
    console.log('PASS: unrelated grants withhold the catalog as denied, not empty, and direct IDs are not found');

    setGrants(catalog);
    setPolicy(2);
    assert.equal((await loadList()).status(), 403);
    await expect(page.getByRole('alert')).toContainText('Could not load clients');
    await expect(page.getByRole('link', { name: 'Permission zero client', exact: true })).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log('PASS: an unknown policy fails closed on the Clients page');
  } catch (error) {
    console.error(error);
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot({ timeout: 3000 }).catch(failure => failure.message) });
    throw error;
  } finally {
    await browser.close();
    if (owned) sql(`BEGIN;
      UPDATE organizations SET permission_policy_version=0 WHERE id='${org}';
      DELETE FROM person_permission_states WHERE id='${state}';
      DELETE FROM project_management_assignments WHERE id='${assignment}';
      DELETE FROM projects WHERE id='${project}';
      DELETE FROM clients WHERE id IN ('${unset}','${zero}'); COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
