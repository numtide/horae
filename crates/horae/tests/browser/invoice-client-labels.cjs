// Fixtures are restricted to the browser runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const clientId = '019a0040-0000-7000-8000-000000000001';
const invoiceId = '019a0040-0000-7000-8000-000000000002';
const name = 'Historical <client> & agency';
sql(`BEGIN;
  INSERT INTO clients (id, org_id, name, currency)
    SELECT '${clientId}', org_id, '${name}', 'EUR' FROM users
    WHERE org_role='admin' ORDER BY id LIMIT 1;
  INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency)
    SELECT '${invoiceId}', org_id, id, 'LABEL-REGRESSION', 'paid', '2026-10-01', '2026-10-31', 'EUR'
    FROM clients WHERE id='${clientId}';
  COMMIT;`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    for (const active of [true, false]) {
      sql(`UPDATE clients SET active=${active} WHERE id='${clientId}'`);
      await page.goto(`${base}/invoices`);
      const row = page.getByRole('row').filter({ hasText: 'LABEL-REGRESSION' });
      await expect(row.locator('td').nth(1)).toHaveText(name);
      await row.getByRole('link', { name: 'View', exact: true }).click();
      await expect(page).toHaveURL(`${base}/invoices/${invoiceId}`);
      await expect(page.locator('main').getByText(name, { exact: true })).toBeVisible();
      await expect(page.locator('main').getByText(clientId, { exact: true })).toHaveCount(0);
      // Direct navigation must work independently of the list's loaded catalog.
      await page.reload();
      await expect(page.locator('main').getByText(name, { exact: true })).toBeVisible();
      await page.goto(`${base}/invoices`);
      await page.getByRole('button', { name: 'New Invoice', exact: true }).click();
      const picker = page.getByLabel('Client', { exact: true });
      await expect(picker).toBeVisible();
      await expect(picker.locator(`option[value="${clientId}"]`)).toHaveCount(active ? 1 : 0);
    }
    assert.deepEqual(errors, []);
    console.log('invoice client labels: active/inactive list, detail, reload and creation picker passed');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
