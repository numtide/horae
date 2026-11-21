// Only run through run-design-checks.sh against its disposable seeded database.
// It submits and approves the seeded week and voids an invoice, so it runs last.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const expect = baseExpect.configure({ timeout: 15000 });
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const base = process.env.HORAE_TEST_URL;
const week = process.env.HORAE_TEST_WEEK;
assert.ok(base && week, 'Set HORAE_TEST_URL and HORAE_TEST_WEEK to an isolated seeded instance');
const database = new URL(process.env.DATABASE_URL);
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const header = page.locator('.page-header');
  // Each screen shows at most one Primary, and it sits in the header. Closed
  // dialogs stay mounted with their own Primary, so count only what is shown.
  const onePrimary = async () => {
    await expect(page.locator('main .btn-primary:visible')).toHaveCount(1);
    await expect(header.locator('.btn-primary')).toHaveCount(1);
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    // Timesheet: "Submit week" is the header's Primary; adding an entry is Secondary.
    await page.goto(`${base}/timesheet/week/${week}`);
    const submit = header.getByRole('button', { name: 'Submit week', exact: true });
    await expect(submit).toHaveClass(/btn-primary/);
    await expect(page.getByRole('button', { name: 'Add entry', exact: true })).toHaveClass(/btn-secondary/);
    await onePrimary();
    await expect(page.locator('.btn-solid, .ts-submit, .ts-add')).toHaveCount(0);
    await page.goto(`${base}/timesheet/day/${week}`);
    const entry = page.locator('.ts-day-entry').first();
    await expect(entry.getByRole('button', { name: 'Edit', exact: true })).toHaveClass(/btn-icon/);
    await expect(entry.getByRole('button', { name: 'Start', exact: true })).toHaveClass(/btn-secondary/);
    await expect(submit).toBeVisible();
    await submit.click();
    await expect(header.getByText('Submitted', { exact: true })).toBeVisible();
    await expect(submit).toHaveCount(0);
    console.log('PASS: Submit week is the Timesheet header Primary in every view; the day row keeps Start and edits from an icon button');

    // Approvals: "Approve all" in the header; each row keeps Approve and Reopen as Secondary.
    await page.goto(`${base}/approvals?week=${week}`);
    const approveAll = header.getByRole('button', { name: 'Approve all', exact: true });
    await expect(approveAll).toHaveClass(/btn-primary/);
    await onePrimary();
    const row = page.getByRole('row').filter({ hasText: 'Admin User' });
    for (const name of ['Approve', 'Reopen']) {
      await expect(row.getByRole('button', { name, exact: true })).toHaveClass(/btn-secondary btn-sm/);
    }
    await expect(page.getByText(/^\d+ timesheets? awaiting your approval$/)).toBeVisible();
    await approveAll.click();
    await expect(page.getByText(/^Approved (\d+) of \1 timesheets$/)).toBeVisible();
    await expect(approveAll).toHaveCount(0);
    console.log('PASS: Approvals approves all from its header and keeps the row decisions inline');

    // Invoices: "New invoice" opens its own screen, whose one Primary is "Generate draft".
    await page.goto(`${base}/invoices`);
    const create = header.getByRole('link', { name: 'New invoice', exact: true });
    await expect(create).toHaveClass(/btn-primary/);
    await create.click();
    await expect(page).toHaveURL(`${base}/invoices/new`);
    await expect(page.getByRole('heading', { name: 'New invoice', exact: true })).toBeVisible();
    await expect(header.getByRole('button')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Generate draft', exact: true })).toHaveClass(/btn-primary/);
    await expect(page.locator('main .btn-primary:visible')).toHaveCount(1);
    // The detail checks need a draft. The invoice suites before this one leave
    // several; on a fresh database, prepare one from the seeded week.
    let id = sql(`SELECT i.id FROM invoices i JOIN clients c ON c.id = i.client_id
      WHERE i.status = 'draft' AND c.active ORDER BY i.created_at DESC LIMIT 1`);
    if (id) {
      await page.goto(`${base}/invoices/${id}`);
    } else {
      const seeded = sql(`SELECT c.name FROM time_entries e JOIN projects p ON p.id = e.project_id
        JOIN clients c ON c.id = p.client_id
        WHERE e.spent_date BETWEEN '${week}'::date AND '${week}'::date + 6 AND e.billable
        GROUP BY c.name ORDER BY count(*) DESC LIMIT 1`);
      await page.getByLabel('Client', { exact: true }).selectOption({ label: seeded });
      await page.getByLabel('Period from', { exact: true }).fill(week);
      await page.getByLabel('Period to', { exact: true }).fill(sql(`SELECT ('${week}'::date + 6)::text`));
      await page.getByRole('button', { name: 'Review invoice', exact: true }).click();
      const generate = page.getByRole('button', { name: 'Generate draft', exact: true });
      await expect(generate).toBeEnabled();
      await generate.click();
      await expect(page).toHaveURL(/\/invoices\/[0-9a-f-]{36}$/);
      id = page.url().split('/').pop();
    }
    const number = sql(`SELECT number FROM invoices WHERE id = '${id}'`);
    const client = sql(`SELECT client_id FROM invoices WHERE id = '${id}'`);
    const clientName = sql(`SELECT name FROM clients WHERE id = '${client}'`);

    // Invoice detail: Mark Sent is the Primary; Edit and Void sit in "Actions", Void last.
    await expect(header.getByRole('button', { name: 'Mark Sent', exact: true })).toHaveClass(/btn-primary/);
    await onePrimary();
    const actions = page.locator('#invoice-actions-trigger');
    await actions.focus();
    await page.keyboard.press('ArrowDown');
    await expect(page.getByRole('menuitem', { name: 'Edit invoice values', exact: true })).toBeFocused();
    await page.keyboard.press('End');
    const voidItem = page.getByRole('menuitem', { name: 'Void…', exact: true });
    await expect(voidItem).toBeFocused();
    await expect(voidItem).toHaveClass(/danger/);
    await page.keyboard.press('Escape');
    await expect(actions).toBeFocused();
    await actions.click();
    await voidItem.click();
    const confirm = page.getByRole('dialog', { name: `Void invoice ${number}?`, exact: true });
    await expect(confirm).toBeVisible();
    await expect(confirm.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(confirm).toBeHidden();
    assert.equal(sql(`SELECT status FROM invoices WHERE id = '${id}'`), 'draft');
    await actions.click();
    await voidItem.click();
    await confirm.getByRole('button', { name: 'Void invoice', exact: true }).click();
    await expect(confirm).toBeHidden();
    await expect(page.locator('main .badge').filter({ hasText: 'Void' })).toBeVisible();
    await expect(actions).toHaveCount(0);
    await expect(header.getByRole('button', { name: 'Mark Sent', exact: true })).toHaveCount(0);
    assert.equal(sql(`SELECT status FROM invoices WHERE id = '${id}'`), 'void');
    console.log('PASS: invoice detail offers Mark Sent as its Primary, Edit and Void… in its Actions menu, and voids only after confirming');

    // Invoice list: the whole row opens the invoice; there is no Actions column.
    await page.goto(`${base}/invoices`);
    await expect(page.getByRole('columnheader', { name: 'Actions', exact: true })).toHaveCount(0);
    const listed = page.getByRole('row').filter({ has: page.getByRole('link', { name: number, exact: true }) });
    await expect(listed.locator('.badge')).toHaveText('Void');
    // The number's link covers the row, so it, not the cell, takes this click.
    await listed.getByRole('cell', { name: clientName, exact: true }).click({ force: true });
    await expect(page).toHaveURL(`${base}/invoices/${id}`);
    console.log('PASS: an invoice row opens its invoice from anywhere in the row');

    // Client detail: New invoice (Primary), New project, then Edit and Archive in "Actions".
    await page.goto(`${base}/clients/${client}`);
    await expect(header.getByRole('link', { name: 'New invoice', exact: true })).toHaveClass(/btn-primary/);
    await expect(header.getByRole('link', { name: 'New project', exact: true })).toHaveClass(/btn-secondary/);
    await onePrimary();
    await page.locator('#client-actions-trigger').click();
    await expect(page.getByRole('menuitem')).toHaveText(['Edit client', 'Archive client']);
    await page.keyboard.press('Escape');
    console.log('PASS: Client detail keeps one Primary and its record actions in a menu');
    assert.deepEqual(errors, [], 'no uncaught browser errors');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exit(1); });
