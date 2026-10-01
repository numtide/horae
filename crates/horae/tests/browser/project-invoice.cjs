// Run through run-design-checks.sh: financial mutations stay in its disposable DB.
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
sql(`BEGIN;
  INSERT INTO clients (id,org_id,name,currency)
    SELECT '01970000-0000-7000-8000-000000000100',id,'Context invoice client','EUR'
    FROM organizations WHERE name='Demo Org';
  INSERT INTO projects (id,org_id,client_id,name,code,currency,rate_cents)
    SELECT fixture.id::uuid,o.id,'01970000-0000-7000-8000-000000000100',
      fixture.name,fixture.code,'EUR',10000
    FROM organizations o CROSS JOIN (VALUES
      ('01970000-0000-7000-8000-000000000101','Context invoice project','CONTEXT-1'),
      ('01970000-0000-7000-8000-000000000102','Other client project','CONTEXT-2')
    ) fixture(id,name,code) WHERE o.name='Demo Org';
  INSERT INTO project_tasks (project_id,task_id,billable,rate_cents)
    SELECT p.id,t.id,true,10000 FROM projects p JOIN tasks t ON t.org_id=p.org_id
    WHERE p.code IN ('CONTEXT-1','CONTEXT-2') AND t.name='Development';
  INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable)
    SELECT CASE p.code WHEN 'CONTEXT-1' THEN '01970000-0000-7000-8000-000000000201'::uuid
      ELSE '01970000-0000-7000-8000-000000000202'::uuid END,
      p.org_id,u.id,p.id,t.id,'2026-09-01',60,true
    FROM projects p JOIN users u ON u.org_id=p.org_id AND u.email='admin@example.com'
    JOIN tasks t ON t.org_id=p.org_id AND t.name='Development'
    WHERE p.code IN ('CONTEXT-1','CONTEXT-2');
  COMMIT;`);
const project = JSON.parse(sql(`SELECT row_to_json(p) FROM (
  SELECT p.id, p.client_id, p.name, c.name AS client_name,
    min(e.spent_date)::text AS date_from, max(e.spent_date)::text AS date_to
  FROM projects p JOIN clients c ON c.id=p.client_id
  JOIN time_entries e ON e.project_id=p.id
  WHERE p.code='CONTEXT-1' GROUP BY p.id,c.name) p`));
const invoicesBefore = Number(sql('SELECT count(*) FROM invoices'));

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/projects/${project.id}`);
    await page.getByRole('tab', { name: /^Invoices/ }).click();
    await page.getByRole('link', { name: 'New invoice', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${project.id}/invoices/new`);
    await expect(page.getByText("Only this project's time and fees will be reviewed.", { exact: true })).toBeVisible();
    await expect(page.locator('dd').filter({ hasText: project.name })).toBeVisible();
    await expect(page.locator('dd').filter({ hasText: project.client_name })).toBeVisible();
    await expect(page.getByRole('combobox', { name: 'Client', exact: true })).toHaveCount(0);
    await expect(page.getByRole('checkbox', { name: 'All projects for this client', exact: true })).toHaveCount(0);
    await expect(page.locator('[data-editor-state]')).toHaveAttribute('data-editor-state', 'clean');
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(url => url.origin === base && url.pathname === `/projects/${project.id}` && url.search === '');
    assert.equal(Number(sql('SELECT count(*) FROM invoices')), invoicesBefore);

    await page.goto(`${base}/projects/${project.id}/invoices/new`);
    const from = page.getByLabel('Period from', { exact: true });
    const to = page.getByLabel('Period to', { exact: true });
    const review = page.getByRole('button', { name: 'Review invoice', exact: true });
    const generate = page.getByRole('button', { name: 'Generate draft', exact: true });
    await from.fill(project.date_from);
    await to.fill(project.date_to);
    await expect(generate).toBeDisabled();
    await page.route('**/api/prepare_invoice*', route => route.abort(), { times: 1 });
    await review.click();
    await expect(page.locator('.alert-danger')).toBeVisible();
    await expect(generate).toBeDisabled();
    const previewResponse = page.waitForResponse(response => response.url().includes('/api/prepare_invoice') && response.status() === 200);
    await review.click();
    const response = await previewResponse;
    const payload = response.request().postDataJSON();
    assert.equal(payload.client_id, project.client_id);
    assert.deepEqual(payload.project_ids, [project.id]);
    const estimate = await response.json();
    assert.deepEqual(estimate.projects.map(p => p.project_id), [project.id]);
    await expect(generate).toBeEnabled();
    assert.equal(Number(sql('SELECT count(*) FROM invoices')), invoicesBefore);
    const dialog = page.waitForEvent('dialog');
    const cancel = page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await (await dialog).accept();
    await cancel;
    await expect(page).toHaveURL(url => url.origin === base && url.pathname === `/projects/${project.id}` && url.search === '');
    assert.equal(Number(sql('SELECT count(*) FROM invoices')), invoicesBefore);

    await page.goto(`${base}/projects/${project.id}/invoices/new`);
    await from.fill(project.date_from);
    await to.fill(project.date_to);
    await review.click();
    await expect(generate).toBeEnabled();
    for (const width of [320, 390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
    }
    const generationResponse = page.waitForResponse(response => response.url().includes('/api/generate_invoice') && response.status() === 200);
    await generate.click();
    const generated = await generationResponse;
    assert.deepEqual(generated.request().postDataJSON().project_ids, [project.id]);
    const result = await generated.json();
    await expect(page).toHaveURL(`${base}/invoices/${result.invoice.id}`);
    assert.equal(Number(sql('SELECT count(*) FROM invoices')), invoicesBefore + 1);
    assert.equal(sql(`SELECT count(*) FROM invoice_line_items l LEFT JOIN time_entries e ON e.id=l.time_entry_id LEFT JOIN project_fee_occurrences f ON f.id=l.fee_occurrence_id WHERE l.invoice_id='${result.invoice.id}' AND coalesce(e.project_id,f.project_id)<>'${project.id}'`), '0');
    let historyReads = 0;
    page.on('request', request => { if (request.url().includes('/api/get_project_invoices')) historyReads++; });
    await page.goto(`${base}/projects/${project.id}`);
    const invoiced = page.getByRole('region', { name: 'Invoiced', exact: true });
    await expect(invoiced).toContainText('EUR 100.00');
    await expect(invoiced).toContainText('Across 1 invoice');
    await expect(invoiced).not.toContainText('Lifetime');
    await page.getByRole('tab', { name: /^Invoices/ }).click();
    await expect(page.getByRole('table', { name: 'Project invoice history', exact: true })).toBeVisible();
    const invoiceTable = page.getByRole('table', { name: 'Project invoice history', exact: true });
    await expect(page.getByRole('heading', { name: 'All invoice history', exact: true })).toHaveCount(0);
    await expect(page.getByRole('tabpanel').locator('dl')).toHaveCount(0);
    await expect(invoiceTable.locator('tfoot')).toHaveText('TotalEUR 100.00');
    assert.equal(historyReads, 1, 'Summary and history share one invoice read');
    for (const width of [320, 390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
      assert.equal(await invoiced.evaluate(element => element.scrollWidth <= element.clientWidth + 1), true);
      assert.equal(await invoiceTable.locator('tbody a').evaluate(element => getComputedStyle(element).whiteSpace), 'nowrap');
      assert.equal(await invoiceTable.locator('..').evaluate(element => getComputedStyle(element).borderRadius), '16px');
    }
    await page.route('**/api/get_project_invoices*', route => route.abort(), { times: 1 });
    // The assertions below wait for the application's error/retry state.
    await page.reload({ waitUntil: 'domcontentloaded' });
    await expect(invoiced.getByRole('alert')).toContainText('Could not load invoiced total');
    await expect(invoiced).not.toContainText('EUR 100.00');
    await invoiced.getByRole('button', { name: 'Retry invoiced total', exact: true }).click();
    await expect(invoiced).toContainText('EUR 100.00');
    await page.getByRole('tab', { name: /^Invoices/ }).click();
    await expect(page.getByRole('table', { name: 'Project invoice history', exact: true })).toContainText('EUR 100.00');
    assert.equal(historyReads, 3, 'A failed read and one retry update both views');
    await page.getByRole('link', { name: result.invoice.number, exact: true }).click();
    await expect(page).toHaveURL(`${base}/invoices/${result.invoice.id}`);
    assert.equal(sql("SELECT state FROM time_entries WHERE id='01970000-0000-7000-8000-000000000202'"), 'open');

    await page.route('**/api/get_project_details*', route => route.abort(), { times: 1 });
    await page.goto(`${base}/projects/${project.id}/invoices/new`);
    await expect(page.getByRole('alert')).toContainText('Could not load invoice project');
    await expect(review).toHaveCount(0);
    await page.getByRole('button', { name: 'Retry project', exact: true }).click();
    await expect(review).toBeVisible();
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(url => url.origin === base && url.pathname === `/projects/${project.id}` && url.search === '');

    const actor = sql("SELECT id FROM users WHERE email='admin@example.com'");
    try {
      sql(`UPDATE users SET org_role='member' WHERE id='${actor}'`);
      let identityRequests = 0;
      page.on('request', request => { if (request.url().includes('/api/get_project_details')) identityRequests++; });
      await page.goto(`${base}/projects/${project.id}/invoices/new`);
      await expect(page.getByText('Invoices require manager access.', { exact: true })).toBeVisible();
      await expect(review).toHaveCount(0);
      await expect(page.getByText(project.name, { exact: true })).toHaveCount(0);
      assert.equal(identityRequests, 0);
      assert.equal(Number(sql('SELECT count(*) FROM invoices')), invoicesBefore + 1);
    } finally {
      sql(`UPDATE users SET org_role='admin' WHERE id='${actor}'`);
    }
    assert.deepEqual(errors, []);
    console.log('Project invoice: scoped generation, cancel/retry, shared invoiced tile/history, responsive bounds and permissions passed');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
