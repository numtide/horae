// The Invoices list, its Uninvoiced view, the detail's Primary per status,
// the unapproved-time warning, Subject and Duplicate. Fixtures are restricted
// to the browser runner's disposable database and use their own client.
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
const id = n => `019a0050-0000-7000-8000-${String(n).padStart(12, '0')}`;
const client = id(1);
const clientName = 'Screens & <Co>';
const project = id(2);
const projectName = 'Screens website';
const task = id(3);
const [draft, sent, paid, voided] = [id(10), id(11), id(12), id(13)];
const [open, submitted, later] = [id(20), id(21), id(22)];
sql(`BEGIN;
  INSERT INTO clients (id, org_id, name, currency)
    SELECT '${client}', org_id, '${clientName}', 'EUR' FROM users WHERE email = 'admin@example.com';
  INSERT INTO projects (id, org_id, client_id, name, currency, rate_cents)
    SELECT '${project}', org_id, '${client}', '${projectName}', 'EUR', 10000 FROM clients WHERE id = '${client}';
  INSERT INTO tasks (id, org_id, name) SELECT '${task}', org_id, 'Screens design' FROM clients WHERE id = '${client}';
  INSERT INTO project_tasks (project_id, task_id, billable) VALUES ('${project}', '${task}', true);
  INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, state)
    SELECT e.id::uuid, u.org_id, u.id, '${project}', '${task}', '2026-11-02', e.minutes, e.notes, true, e.state::entry_state
    FROM users u, (VALUES ('${open}', 120, 'Open hours', 'open'), ('${submitted}', 90, 'Waiting hours', 'submitted')) AS e(id, minutes, notes, state)
    WHERE u.email = 'admin@example.com';
  INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents)
    SELECT i.id::uuid, c.org_id, c.id, i.number, i.status::invoice_status, i.issued::date, i.due::date, 'EUR', i.total
    FROM clients c, (VALUES
      ('${draft}', 'SCREENS-1', 'draft', '2026-01-02', '2026-02-01', 1000),
      ('${sent}', 'SCREENS-2', 'sent', '2026-01-01', '2026-01-31', 2000),
      ('${paid}', 'SCREENS-3', 'paid', '2025-06-01', '2025-07-01', 3000),
      ('${voided}', 'SCREENS-4', 'void', '2025-05-01', '2025-05-31', 4000)) AS i(id, number, status, issued, due, total)
    WHERE c.id = '${client}';
  COMMIT;`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const header = page.locator('.page-header');
  const rows = page.locator('main tbody tr');
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    // Loading: a skeleton under a live header, never a blank screen.
    let release;
    const held = new Promise(resolve => { release = resolve; });
    await page.route('**/api/list_invoices*', async route => { await held; await route.continue(); }, { times: 1 });
    await page.goto(`${base}/invoices`);
    await expect(header.getByRole('link', { name: 'New invoice', exact: true })).toBeVisible();
    await expect(page.locator('.skeleton').getByRole('status')).toHaveText('Loading invoices…');
    release();
    await expect(page.locator('.skeleton')).toHaveCount(0);

    // Filters: search, status (Overdue is computed), period, then no match.
    const search = page.getByRole('searchbox', { name: 'Search by number or client', exact: true });
    await search.fill('SCREENS');
    await expect(rows).toHaveCount(4);
    await page.locator('#invoices-status-filter-trigger').click();
    await page.getByRole('menuitem', { name: 'Overdue', exact: true }).click();
    await expect(rows).toHaveCount(1);
    await expect(rows.first()).toContainText('SCREENS-2');
    await expect(rows.first().locator('.badge')).toHaveText('Overdue');
    await expect(rows.first().locator('td.text-danger')).toHaveText('31 Jan 2026');
    await page.locator('#invoices-status-filter-trigger').click();
    await page.getByRole('menuitem', { name: 'Void', exact: true }).click();
    await expect(rows).toHaveCount(1);
    await expect(rows.first().locator('.inv-void-amount')).toHaveText('EUR 40.00');
    await page.locator('#invoices-status-filter-trigger').click();
    await page.getByRole('menuitem', { name: 'Any status', exact: true }).click();
    await page.locator('#invoices-period-filter-trigger').click();
    await page.getByRole('menuitem', { name: 'Last year', exact: true }).click();
    await expect(rows).toHaveCount(2);
    await search.fill('SCREENS-404');
    await expect(page.getByRole('heading', { name: 'No invoices match these filters', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Clear filters', exact: true }).click();
    await expect(search).toHaveValue('');
    await expect(page.locator('#invoices-period-filter-trigger')).toContainText('Any time');
    // Summary: overdue counts what is past due, in its own currency.
    await expect(page.locator('.metric-card').filter({ hasText: 'Overdue' })).toContainText('EUR');
    console.log('PASS: invoice list loads behind a skeleton and filters by status, overdue, period and search');

    // Rows link to the invoice; the Primary follows the status.
    await search.fill('SCREENS-2');
    await rows.first().getByRole('cell', { name: clientName, exact: true }).click({ force: true });
    await expect(page).toHaveURL(`${base}/invoices/${sent}`);
    await expect(header.locator('.badge')).toHaveText('Overdue');
    await expect(header.getByRole('button', { name: 'Mark as paid', exact: true })).toHaveClass(/btn-primary/);
    await expect(header.locator('.btn-primary')).toHaveCount(1);
    await page.goto(`${base}/invoices/${draft}`);
    await expect(header.getByRole('button', { name: 'Mark as sent', exact: true })).toHaveClass(/btn-primary/);
    await page.goto(`${base}/invoices/${paid}`);
    await expect(header.locator('.badge')).toHaveText('Paid');
    await expect(header.getByRole('link', { name: 'Download PDF', exact: true })).toBeVisible();
    await expect(header.locator('.btn-primary')).toHaveCount(0);
    console.log('PASS: rows open the invoice and the Primary is Mark as sent, Mark as paid or none by status');

    // Uninvoiced: the client's unbilled time, with its own New invoice.
    await page.goto(`${base}/invoices`);
    await page.waitForLoadState('networkidle');
    await page.getByRole('button', { name: 'Uninvoiced', exact: true }).click();
    await expect(page.getByRole('columnheader', { name: 'Unbilled time', exact: true })).toBeVisible();
    const unbilled = page.getByRole('row').filter({ hasText: clientName });
    await expect(unbilled).toContainText('2:00');
    await expect(unbilled).toContainText('EUR 200.00');
    await unbilled.getByRole('link', { name: `New invoice for ${clientName}`, exact: true }).click();
    await expect(page).toHaveURL(`${base}/invoices/new/client/${client}`);

    // Step 2 warns about time waiting for approval, which stays out.
    const warning = page.locator('#inv-awaiting-approval');
    await expect(warning).toContainText('1:30 of billable time');
    await expect(warning.getByRole('link', { name: 'Open Approvals', exact: true })).toHaveAttribute('href', /^\/approvals/);
    const preview = page.getByRole('region', { name: 'Preview', exact: true });
    await expect(preview.locator('tbody tr')).toHaveCount(1);
    await expect(preview.locator('tbody')).toContainText('Open hours');
    sql(`UPDATE time_entries SET state = 'approved' WHERE id = '${submitted}'`);
    await page.reload();
    await expect(preview.locator('tbody tr')).toHaveCount(2);
    await expect(warning).toHaveCount(0);
    console.log('PASS: Uninvoiced opens step 2 for its client, which warns about unapproved time and leaves it out');

    // Create the draft, give it a subject, and see it on the detail.
    await page.getByRole('button', { name: 'Create draft', exact: true }).click();
    await expect(page).toHaveURL(/\/invoices\/[0-9a-f-]{36}\/edit$/);
    const created = page.url().split('/').at(-2);
    await page.getByLabel('Subject', { exact: true }).fill('Website, November');
    await page.getByRole('button', { name: 'Save draft', exact: true }).click();
    await expect(page.getByRole('status').filter({ hasText: 'Draft saved' })).toBeVisible();
    await page.getByRole('link', { name: 'Back to invoice', exact: true }).click();
    await expect(page.locator('main')).toContainText('Website, November');
    assert.equal(sql(`SELECT subject FROM invoices WHERE id = '${created}'`), 'Website, November');

    // Duplicate starts step 2 from this invoice; its time is never copied.
    await page.locator('#invoice-actions-trigger').click();
    await page.locator('#invoice-duplicate').click();
    await expect(page).toHaveURL(`${base}/invoices/new/client/${client}/from/${created}`);
    await expect(page.locator('main')).toContainText('Starting from invoice');
    await expect(page.getByRole('region', { name: 'Projects', exact: true }).getByRole('checkbox', { name: projectName, exact: true })).toHaveAttribute('aria-checked', 'true');
    await expect(page.locator('.alert-danger')).toContainText('No billable time or fees found');
    await expect(page.getByRole('button', { name: 'Create draft', exact: true })).toBeDisabled();
    sql(`INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable)
      SELECT '${later}', org_id, id, '${project}', '${task}', '2026-11-03', 30, 'Later hours', true FROM users WHERE email = 'admin@example.com'`);
    await page.getByRole('button', { name: 'Try again', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Create draft', exact: true })).toBeEnabled();
    await page.getByRole('button', { name: 'Create draft', exact: true }).click();
    await expect(page).toHaveURL(/\/invoices\/[0-9a-f-]{36}\/edit$/);
    const copy = page.url().split('/').at(-2);
    assert.notEqual(copy, created);
    assert.equal(sql(`SELECT subject FROM invoices WHERE id = '${copy}'`), 'Website, November');
    assert.equal(sql(`SELECT string_agg(time_entry_id::text, ',') FROM invoice_line_items WHERE invoice_id = '${copy}'`), later);
    assert.equal(sql(`SELECT invoice_id FROM time_entries WHERE id = '${open}'`), created);
    console.log('PASS: Subject is saved and shown; Duplicate copies values and subject into a new draft without billed time');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error(error);
    console.error({ url: page.url(), errors });
    throw error;
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
