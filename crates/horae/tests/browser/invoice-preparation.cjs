// Fee/conflict fixtures are restricted to the runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
sql(`BEGIN;
  UPDATE projects SET client_id = (SELECT id FROM clients WHERE name = 'Acme Corp'),
    currency = 'EUR', project_type = 'fixed_fee', starts_on = '2026-09-01'
    WHERE code IN ('ACME-01', 'TECH-01');
  INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode,
    fee_mode, fee_amount_cents, terms_days, tax1_bps, po_number)
    SELECT id, org_id, id, (SELECT id FROM users WHERE org_role = 'admin' ORDER BY id LIMIT 1),
      'person', 'single', CASE code WHEN 'ACME-01' THEN 12500 ELSE 90000 END,
      CASE code WHEN 'ACME-01' THEN 14 ELSE 90 END,
      CASE code WHEN 'ACME-01' THEN 2100 ELSE 0 END, code
    FROM projects WHERE code IN ('ACME-01', 'TECH-01');
  COMMIT;`);
const project = JSON.parse(sql(`SELECT row_to_json(p) FROM (SELECT id, name FROM projects WHERE code = 'ACME-01') p`));
const invoicesBefore = Number(sql('SELECT count(*) FROM invoices'));
const feesBefore = Number(sql('SELECT count(*) FROM project_fee_occurrences'));

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  const pending = new Set();
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) pending.add(request); });
  page.on('requestfinished', request => pending.delete(request));
  page.on('requestfailed', request => pending.delete(request));
  const readsFinished = () => expect.poll(() => pending.size).toBe(0);
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.waitForLoadState('networkidle');
    const clientsLoaded = page.waitForResponse(response => response.url().includes('/api/list_clients') && response.status() === 200);
    await page.getByRole('link', { name: 'Invoices', exact: true }).click();
    await (await clientsLoaded).finished();
    await readsFinished();
    await page.getByRole('button', { name: 'New Invoice', exact: true }).click();
    await page.getByLabel('Client', { exact: true }).selectOption({ label: 'Acme Corp' });
    await page.getByLabel('Period from', { exact: true }).fill('2026-09-01');
    await page.getByLabel('Period to', { exact: true }).fill('2026-09-30');
    const review = page.getByRole('button', { name: 'Review invoice', exact: true });
    const generate = page.getByRole('button', { name: 'Generate draft', exact: true });
    await review.click();
    await expect(page.getByRole('status').filter({ hasText: 'different invoice defaults' })).toBeVisible();
    await expect(generate).toBeDisabled();
    await expect(page.getByLabel('Days until payment is due', { exact: true })).toHaveValue('');
    await page.getByRole('button', { name: `Use defaults from ${project.name}`, exact: true }).click();
    await expect(page.getByLabel('Days until payment is due', { exact: true })).toHaveValue('14');
    await expect(generate).toBeDisabled();
    await review.click();
    await expect(generate).toBeEnabled();
    const charges = page.getByRole('table', { name: 'Estimated invoice charges (EUR)', exact: true });
    await expect(charges.locator('tbody tr')).toHaveCount(2);
    await expect(charges.locator('tfoot tr').last()).toHaveText(/TotalEUR 1240.25/);
    for (const row of await charges.locator('tbody tr').all()) {
      await expect(row.locator('td').nth(1)).toHaveText('—');
      await expect(row.locator('td').nth(2)).toHaveText('—');
    }
    assert.equal(Number(sql('SELECT count(*) FROM invoices')), invoicesBefore);
    assert.equal(Number(sql('SELECT count(*) FROM project_fee_occurrences')), feesBefore);
    await page.getByRole('checkbox', { name: 'All projects for this client', exact: true }).click();
    await expect(generate).toBeDisabled();
    await review.click();
    await expect(page.getByText(/Select between 1 and 1000 projects/)).toBeVisible();
    await page.getByRole('checkbox', { name: project.name, exact: true }).click();
    await page.getByLabel('PO number', { exact: true }).fill('Fee review retry');
    await page.route('**/api/prepare_invoice*', route => route.abort());
    await review.click();
    await expect(page.locator('.alert-danger')).toBeVisible();
    await expect(review).toBeEnabled();
    await expect(generate).toBeDisabled();
    await expect(page.getByLabel('PO number', { exact: true })).toHaveValue('Fee review retry');
    await readsFinished();
    await page.unroute('**/api/prepare_invoice*');
    await review.click();
    await expect(generate).toBeEnabled();
    await expect(charges.locator('tbody tr')).toHaveCount(1);
    await expect(charges.locator('tfoot tr').last()).toHaveText(/TotalEUR 151.25/);
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
    }
    const generated = page.waitForResponse(response => response.url().includes('/api/generate_invoice') && response.status() === 200);
    await generate.click();
    const result = await (await generated).json();
    assert.deepEqual([result.invoice.terms_days, result.invoice.po_number, result.invoice.total_cents], [14, 'Fee review retry', 15125]);
    assert.equal(result.lines.length, 1);
    assert.equal(result.lines[0].minutes, null);
    assert.ok(result.lines[0].fee_occurrence_id);
    await expect(page).toHaveURL(`${base}/invoices/${result.invoice.id}`);
    await expect(page.getByRole('button', { name: 'Edit invoice values', exact: true })).toBeVisible();
    await readsFinished();
    assert.equal(Number(sql('SELECT count(*) FROM invoices')), invoicesBefore + 1);
    assert.equal(Number(sql('SELECT count(*) FROM project_fee_occurrences')), feesBefore + 1);
    assert.deepEqual(errors, []);
    console.log('PASS: mixed project defaults require explicit resolution; fee estimates are read-only, responsive and recoverable; generation claims only selected projects');
  } catch (error) {
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot() });
    throw error;
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
