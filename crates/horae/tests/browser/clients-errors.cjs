// Transport failures are injected only into the disposable browser-test instance.
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

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const client = sql("SELECT id FROM clients WHERE name = 'Acme Corp'");
  const original = sql(`SELECT row_to_json(c) FROM clients c WHERE id = '${client}'`);
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    let release;
    const gate = new Promise(resolve => { release = resolve; });
    await page.route('**/api/list_client_summaries*', async route => { await gate; await route.abort('failed'); });
    await page.goto(`${base}/clients`);
    await expect(page.getByRole('status')).toContainText('Loading clients');
    await expect(page.getByText('No clients yet', { exact: true })).toHaveCount(0);
    release();
    await expect(page.getByRole('alert')).toContainText('Could not load clients');
    await page.unroute('**/api/list_client_summaries*');
    await page.getByRole('button', { name: 'Retry clients', exact: true }).click();
    await expect(page.getByRole('link', { name: 'Acme Corp', exact: true })).toBeVisible();
    const search = page.getByRole('searchbox', { name: 'Search clients by name' });
    await search.fill('  aCmE  ');
    await expect(page.getByRole('link', { name: 'Acme Corp', exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'TechStart Inc', exact: true })).toHaveCount(0);
    await search.fill('no-client-with-this-name');
    await expect(page.getByRole('heading', { name: 'No clients match your filters', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Clear filters', exact: true }).click();
    await expect(search).toHaveValue('');
    await expect(page.getByRole('link', { name: 'Acme Corp', exact: true })).toBeVisible();
    sql(`INSERT INTO clients (id, org_id, name, currency, active)
      SELECT '01970000-0000-7000-8000-000000000421', org_id, 'State filter active', 'CHF', true FROM clients WHERE id = '${client}';
      INSERT INTO clients (id, org_id, name, currency, active)
      SELECT '01970000-0000-7000-8000-000000000422', org_id, 'State filter inactive', 'GBP', false FROM clients WHERE id = '${client}'`);
    await page.reload();
    await expect(page.getByRole('link', { name: 'State filter active', exact: true })).toBeVisible();
    await search.fill('State filter');
    await expect(search).toHaveValue('State filter');
    await expect(page.getByRole('link', { name: 'Acme Corp', exact: true })).toHaveCount(0);
    await page.locator('#client-scope-menu-trigger').click();
    await expect(page.getByRole('menuitem', { name: 'Active clients (1)', exact: true })).toBeVisible();
    await expect(page.getByRole('menuitem', { name: 'Inactive clients (1)', exact: true })).toBeVisible();
    await page.getByRole('menuitem', { name: 'All clients (2)', exact: true }).click();
    await page.locator('#client-currency-menu-trigger').click();
    await page.getByRole('menuitem', { name: 'GBP', exact: true }).click();
    await expect(page.getByRole('link', { name: 'State filter inactive', exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'State filter active', exact: true })).toHaveCount(0);
    await page.locator('#client-scope-menu-trigger').click();
    await page.getByRole('menuitem', { name: 'Active clients (0)', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'No clients match your filters', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Clear filters', exact: true }).click();
    await expect(search).toHaveValue('');
    await expect(page.locator('#client-currency-menu-trigger')).toContainText('All currencies');
    await expect(page.getByRole('link', { name: 'State filter inactive', exact: true })).toBeVisible();
    // Explicitly injected empty payload checks presentation, not database emptiness.
    await page.route('**/api/list_client_summaries*', route => route.fulfill({ status: 200, contentType: 'application/json', body: '[]' }));
    await page.reload();
    await expect(page.getByRole('heading', { name: 'No clients yet', exact: true })).toBeVisible();
    await page.unroute('**/api/list_client_summaries*');
    console.log('PASS: clients loading/error/retry and distinct empty/no-match states');

    for (const scenario of [
      { endpoint: 'get_client_details', retry: 'Retry client', message: 'Could not load this client' },
      { endpoint: 'list_projects', retry: 'Retry projects', message: 'Could not load projects' },
      { endpoint: 'list_project_spend', retry: 'Retry totals', message: 'Could not load project totals' },
      { endpoint: 'list_client_invoices', retry: 'Retry invoices', message: 'Could not load invoices' },
    ]) {
      const pattern = `**/api/${scenario.endpoint}*`;
      await page.route(pattern, route => route.abort('failed'));
      await page.goto(`${base}/clients/${client}`);
      await expect(page.getByRole('alert')).toContainText(scenario.message);
      if (scenario.endpoint !== 'get_client_details') {
        await expect(page.getByRole('heading', { name: 'Acme Corp', exact: true })).toBeVisible();
        await expect(page.getByRole('region', { name: 'Billing', exact: true })).toBeVisible();
      }
      await page.unroute(pattern);
      await page.getByRole('button', { name: scenario.retry, exact: true }).click();
      await expect(page.getByRole('alert')).toHaveCount(0);
      await expect(page.getByRole('link', { name: /Acme Website Redesign/ })).toBeVisible();
    }
    console.log('PASS: client identity and independent project/totals/invoice panels recover without false empty results');

    await page.route('**/api/get_client_details*', route => route.abort('failed'));
    await page.getByRole('button', { name: 'Edit client', exact: true }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog.getByRole('alert')).toContainText('Could not load this client for editing');
    await page.unroute('**/api/get_client_details*');
    await dialog.getByRole('button', { name: 'Retry editor', exact: true }).click();
    await dialog.getByLabel('Client name', { exact: true }).fill('Retain failed edit');
    await page.route('**/api/update_client_profile*', route => route.abort('failed'));
    await dialog.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(dialog.getByRole('alert')).toContainText('Could not save client');
    await expect(dialog.getByLabel('Client name', { exact: true })).toHaveValue('Retain failed edit');
    await expect(dialog.getByRole('alert')).toBeFocused();
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.unroute('**/api/update_client_profile*');
    assert.equal(sql(`SELECT row_to_json(c) FROM clients c WHERE id = '${client}'`), original);
    console.log('PASS: failed editor loading can retry and a failed save retains input without changing persisted data');

    await page.route('**/api/project_creation_client*', route => route.abort('failed'));
    await page.goto(`${base}/projects/new/client/${client}`);
    await expect(page.getByRole('alert')).toContainText('Could not load project settings');
    await page.unroute('**/api/project_creation_client*');
    await page.getByRole('button', { name: 'Retry', exact: true }).click();
    await expect(page.locator('#np-client')).toContainText('Acme Corp');
    await expect(page.locator('.np-page header').getByRole('status')).toContainText('Draft saved at');
    await page.locator('.np-page').getByRole('button', { name: 'Discard draft', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Discard draft', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.route('**/api/list_clients*', route => route.abort('failed'));
    await page.goto(`${base}/invoices/new/client/${client}`);
    await expect(page.getByRole('alert')).toBeVisible();
    await page.unroute('**/api/list_clients*');
    await page.getByRole('button', { name: 'Retry clients', exact: true }).click();
    await expect(page.locator('#inv-client')).toHaveValue(client);
    console.log('PASS: contextual project/invoice client loading retries preserve the requested client');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
