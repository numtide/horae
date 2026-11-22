// An invoice imported from Harvest (spec 025): its detail shows Harvest's
// lines, rates and what is paid and due, offers no editor or Duplicate, and
// can be written off. Fixtures are restricted to the browser runner's
// disposable database and use their own client.
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
const id = n => `019a0051-0000-7000-8000-${String(n).padStart(12, '0')}`;
const [client, project, task, invoice, entry] = [id(1), id(2), id(3), id(10), id(20)];
// Lines 1000.00 and a -100.00 credit, 80.00 tax: 980.00, of which 380.00 is due.
sql(`BEGIN;
  INSERT INTO clients (id, org_id, name, currency)
    SELECT '${client}', org_id, 'Imported Co', 'EUR' FROM users WHERE email = 'admin@example.com';
  INSERT INTO projects (id, org_id, client_id, name, currency)
    SELECT '${project}', org_id, '${client}', 'Imported website', 'EUR' FROM clients WHERE id = '${client}';
  INSERT INTO tasks (id, org_id, name) SELECT '${task}', org_id, 'Imported design' FROM clients WHERE id = '${client}';
  INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency,
                        total_cents, tax1_cents, subject, source)
    SELECT '${invoice}', org_id, '${client}', 'HV-1042', 'sent', '2026-10-01', '2099-01-31', 'EUR',
           98000, 8000, 'September work', 'harvest' FROM clients WHERE id = '${client}';
  INSERT INTO harvest_invoices (invoice_id, org_id, harvest_invoice_id, due_cents, tax1_rate, harvest_updated_at)
    SELECT '${invoice}', org_id, 1042, 38000, 8.875, now() FROM clients WHERE id = '${client}';
  INSERT INTO harvest_invoice_lines (id, org_id, invoice_id, position, harvest_line_id, kind, description,
                                     quantity, unit_price_cents, amount_cents, taxed, taxed2)
    SELECT l.id::uuid, c.org_id, '${invoice}', l.position, l.position, l.kind, l.description,
           l.quantity, l.price, l.amount, true, false
    FROM clients c, (VALUES ('${id(11)}', 0, 'Service', 'Design sprint', 12.5, 8000, 100000),
                            ('${id(12)}', 1, 'Credit', 'Goodwill credit', 1, -10000, -10000))
         AS l(id, position, kind, description, quantity, price, amount)
    WHERE c.id = '${client}';
  INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable,
                            state, invoice_id, rounded_minutes)
    SELECT '${entry}', org_id, id, '${project}', '${task}', '2026-09-28', 60, true, 'invoiced', '${invoice}', 60
    FROM users WHERE email = 'admin@example.com';
  COMMIT;`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const header = page.locator('.page-header');
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    await page.goto(`${base}/invoices/${invoice}`);
    await expect(header).toContainText('HV-1042');
    await expect(header.locator('.badge')).toHaveText(['Sent', 'From Harvest']);
    await expect(page.locator('main')).toContainText('was imported from Harvest');
    const lines = page.locator('main tbody tr');
    await expect(lines).toHaveCount(2);
    await expect(lines.nth(0)).toContainText('Design sprint');
    await expect(lines.nth(0)).toContainText('Service');
    await expect(lines.nth(0)).toContainText('12.5');
    await expect(lines.nth(1)).toContainText('Goodwill credit');
    const totals = page.locator('main tfoot');
    for (const label of ['Subtotal', 'Tax (8.875%)', 'Total', 'Paid', 'Amount due']) {
      await expect(totals).toContainText(label);
    }
    console.log('PASS: an imported invoice shows its Harvest lines, rates as written, paid and due');

    // Read only: no editor and no Duplicate; written off, it is no longer due.
    await page.locator('#invoice-actions-trigger').click();
    await expect(page.locator('#invoice-edit')).toHaveCount(0);
    await expect(page.locator('#invoice-duplicate')).toHaveCount(0);
    await expect(page.locator('#invoice-void')).toBeVisible();
    await page.locator('#invoice-write-off').click();
    await expect(page.getByRole('status').filter({ hasText: 'Invoice written off' })).toBeVisible();
    await expect(header.locator('.badge').first()).toHaveText('Written off');
    await expect(page.getByRole('button', { name: 'Mark as paid', exact: true })).toHaveCount(0);
    assert.equal(sql(`SELECT status FROM invoices WHERE id = '${invoice}'`), 'closed');
    assert.equal(sql(`SELECT state || ':' || invoice_id FROM time_entries WHERE id = '${entry}'`), `invoiced:${invoice}`);
    console.log('PASS: an imported invoice offers no editor or Duplicate, and writing it off keeps its time invoiced');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error(error);
    console.error({ url: page.url(), errors });
    throw error;
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
