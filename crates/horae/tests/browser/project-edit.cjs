// All writes belong to run-design-checks.sh's disposable database, never a preview.
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
const project = JSON.parse(sql(`SELECT row_to_json(p) FROM (
  SELECT p.id, p.name, p.code, p.currency, c.name AS client_name
  FROM projects p JOIN clients c ON c.id = p.client_id
  JOIN organizations o ON o.id = p.org_id
  WHERE o.name = 'Demo Org' AND p.code = 'ACME-01'
) p`));
assert.match(project.id, /^[0-9a-f-]{36}$/);
const before = sql(`SELECT json_build_array(
  (SELECT count(*) FROM projects),
  (SELECT count(*) FROM project_drafts),
  (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM time_entries t),
  (SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM invoices i),
  (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l)
)`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  const errors = [];
  let draftWrites = 0;
  page.on('request', request => {
    if (request.url().includes('/api/save_project_draft')) draftWrites++;
  });
  page.on('pageerror', error => errors.push(error.stack || error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`${base}/projects`);
      const row = page.locator('.proj-row').filter({ has: page.locator(`a[href="/projects/${project.id}"]`) });
      await row.getByRole('button', { name: /^Actions/ }).click();
      await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
      await expect(page).toHaveURL(`${base}/projects/${project.id}/edit`);
      const editor = page.locator('.np-page');
      await expect(editor.getByRole('heading', { name: 'Edit project', exact: true })).toBeVisible();
      await expect(page.locator('#proj-name')).toHaveCount(0);
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(project.name);
      await expect(editor.locator('#np-code')).toHaveValue(project.code);
      await expect(editor.getByRole('button', { name: 'Client', exact: true })).toContainText(project.client_name);
      await expect(editor.getByRole('button', { name: 'Currency', exact: true })).toContainText(project.currency);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);

      await editor.getByLabel('Project name', { exact: true }).fill('Cancelled project edit');
      await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
      const confirmation = page.getByRole('dialog');
      await expect(confirmation).toBeVisible();
      await confirmation.getByRole('button', { name: 'Discard changes', exact: true }).click();
      await expect(page).not.toHaveURL(`${base}/projects/${project.id}/edit`);
      assert.equal(sql(`SELECT name FROM projects WHERE id = '${project.id}'`), project.name);

      await page.goto(`${base}/projects/${project.id}/edit`);
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(project.name);
      const name = `Shared project editor ${width}`;
      await editor.getByLabel('Project name', { exact: true }).fill(name);
      await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
      await expect(page).toHaveURL(`${base}/projects/${project.id}`);
      assert.equal(sql(`SELECT name FROM projects WHERE id = '${project.id}'`), name);
      project.name = name;
      await page.goto(`${base}/projects/${project.id}/edit`);
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(name);
      await page.reload();
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(name);
      console.log(`PASS: shared editor prefills, cancels, saves the same project and survives reload at ${width}px`);
    }
    assert.equal(sql(`SELECT json_build_array(
      (SELECT count(*) FROM projects),
      (SELECT count(*) FROM project_drafts),
      (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM time_entries t),
      (SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM invoices i),
      (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l)
    )`), before, 'Editing must not create projects/drafts or rewrite historical time/invoices');
    assert.equal(draftWrites, 0, 'Existing-project editing must never autosave a creation draft');

    await page.goto(`${base}/projects/new`);
    const editor = page.locator('.np-page');
    await editor.locator('#np-client').click();
    await page.getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await editor.getByLabel('Project name', { exact: true }).fill('Configured edit fixture');
    await editor.locator('#np-code').fill('EDIT-CONFIG');
    await editor.locator('#np-notes').fill('Private context before editing');
    await editor.getByRole('radio', { name: /^Project hourly rate/ }).check();
    await editor.locator('#np-project-rate').fill('0');
    await editor.locator('#np-budget-mode').click();
    await page.getByRole('option', { name: 'Total project hours', exact: true }).click();
    await editor.locator('#np-budget-value').fill('10:30');
    await editor.getByRole('button', { name: 'Development', exact: true }).click();
    await editor.getByRole('button', { name: 'Add everyone', exact: true }).click();
    await expect(editor.getByRole('button', { name: 'Add everyone', exact: true })).toBeEnabled();
    await editor.locator('#np-po-number').fill('PO-BEFORE');
    await expect(editor.locator('header').getByRole('status')).toContainText('Draft saved at');
    await editor.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}$/);
    const configuredId = new URL(page.url()).pathname.split('/').pop();
    const draftWritesAfterCreate = draftWrites;
    const configuredBefore = sql(`SELECT json_build_array(
      (SELECT count(*) FROM projects),
      (SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM project_drafts d),
      (SELECT jsonb_agg(to_jsonb(t) ORDER BY task_id) FROM project_tasks t WHERE project_id = '${configuredId}'),
      (SELECT jsonb_agg(jsonb_build_array(id, user_id, created_at) ORDER BY id) FROM assignments WHERE project_id = '${configuredId}')
    )`);
    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await expect(editor.locator('#np-code')).toHaveValue('EDIT-CONFIG');
    await expect(editor.locator('#np-notes')).toHaveValue('Private context before editing');
    await expect(editor.getByRole('radio', { name: /^Project hourly rate/ })).toBeChecked();
    await expect(editor.locator('#np-project-rate')).toHaveValue('0.00');
    await expect(editor.locator('#np-budget-value')).toHaveValue('10:30');
    await expect(editor.locator('#np-po-number')).toHaveValue('PO-BEFORE');
    await expect(editor.getByRole('button', { name: 'Discard draft', exact: true })).toHaveCount(0);
    await editor.locator('#np-code').fill('EDIT-AFTER');
    await editor.locator('#np-project-rate').fill('37.25');
    await editor.locator('#np-budget-value').fill('20:45');
    await editor.locator('#np-notes').fill('Private context after editing');
    await editor.locator('#np-po-number').fill('PO-AFTER');
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${configuredId}`);
    assert.equal(sql(`SELECT code || ':' || rate_cents || ':' || budget_minutes FROM projects WHERE id = '${configuredId}'`), 'EDIT-AFTER:3725:1245');
    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-project-rate')).toHaveValue('37.25');
    await expect(editor.locator('#np-notes')).toHaveValue('Private context after editing');
    await expect(editor.locator('#np-po-number')).toHaveValue('PO-AFTER');
    const revision = sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`);
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${configuredId}`);
    assert.equal(sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`), revision, 'Unchanged save must remain a no-op');
    assert.equal(sql(`SELECT json_build_array(
      (SELECT count(*) FROM projects),
      (SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM project_drafts d),
      (SELECT jsonb_agg(to_jsonb(t) ORDER BY task_id) FROM project_tasks t WHERE project_id = '${configuredId}'),
      (SELECT jsonb_agg(jsonb_build_array(id, user_id, created_at) ORDER BY id) FROM assignments WHERE project_id = '${configuredId}')
    )`), configuredBefore, 'Configured editing preserves project, assignment and task identities and creation drafts');
    assert.equal(draftWrites, draftWritesAfterCreate);
    console.log('PASS: configured editor prefills exact zero, budgets, private notes and invoice defaults; persists changes and preserves identity on no-op');
    await page.goto(`${base}/projects`);
    const configuredRow = page.locator('.proj-row').filter({ has: page.locator(`a[href="/projects/${configuredId}"]`) });
    await configuredRow.getByRole('button', { name: /^Actions/ }).click();
    await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.locator('#np-name').fill('Keep this unsaved edit');
    const dismiss = async action => {
      const dialog = page.waitForEvent('dialog', { timeout: 5000 });
      const navigation = action();
      await (await dialog).dismiss();
      await navigation;
      await expect(page).toHaveURL(`${base}/projects/${configuredId}/edit`);
      await expect(editor.locator('#np-name')).toHaveValue('Keep this unsaved edit');
    };
    await dismiss(() => page.locator('a.nav-item[href="/clients"]').click());
    await dismiss(() => page.evaluate(() => history.back()));
    await dismiss(() => page.evaluate(() => location.reload()));
    await dismiss(() => page.evaluate(() => history.replaceState(history.state, '', '/clients')));
    page.once('dialog', dialog => dialog.accept());
    await page.evaluate(() => history.back());
    await expect(page).toHaveURL(`${base}/projects`);
    await page.evaluate(() => history.forward());
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.evaluate(() => history.back());
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.locator('#np-name').fill('Keep this unsaved edit');
    await dismiss(() => page.evaluate(() => history.forward()));
    await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    assert.equal(sql(`SELECT name FROM projects WHERE id = '${configuredId}'`), 'Configured edit fixture');
    console.log('PASS: sidebar, back, forward, reload and replacement navigation preserve dirty edits until explicitly discarded');

    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.locator('#np-name').fill('Committed with a lost acknowledgement');
    const requests = [];
    await page.route('**/api/save_project_editor*', async route => {
      requests.push(route.request().postData());
      await route.fetch();
      await route.abort('failed');
    });
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(editor.getByRole('alert')).toBeVisible();
    assert.equal(sql(`SELECT name FROM projects WHERE id = '${configuredId}'`), 'Committed with a lost acknowledgement');
    const savedRevision = sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`);
    await expect(editor.locator('#np-name')).toBeDisabled();
    await expect(editor.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    const blockedNavigation = page.waitForEvent('dialog');
    const sidebarClick = page.locator('a.nav-item[href="/clients"]').click();
    const pendingDialog = await blockedNavigation;
    assert.equal(pendingDialog.type(), 'alert');
    await pendingDialog.accept();
    await sidebarClick;
    await expect(page).toHaveURL(`${base}/projects/${configuredId}/edit`);
    await page.unroute('**/api/save_project_editor*');
    await page.route('**/api/save_project_editor*', route => {
      requests.push(route.request().postData());
      return route.continue();
    });
    await editor.getByRole('button', { name: 'Retry request', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(`${base}/projects/${configuredId}`);
    assert.equal(requests.length, 2);
    assert.equal(requests[0], requests[1], 'Retry reuses the exact operation and snapshot');
    assert.equal(sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`), savedRevision);
    await page.unroute('**/api/save_project_editor*');
    console.log('PASS: lost save acknowledgement blocks navigation and retries the identical committed request without another update');

    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-name')).toHaveValue('Committed with a lost acknowledgement');
    await editor.locator('#np-name').fill('Local conflicting changes');
    const otherTab = await page.context().newPage();
    await otherTab.goto(`${base}/projects/${configuredId}/edit`);
    await otherTab.locator('#np-name').fill('Saved from another tab');
    await otherTab.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(otherTab).toHaveURL(`${base}/projects/${configuredId}`);
    await otherTab.close();
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(editor.getByRole('alert')).toBeVisible();
    await expect(editor.locator('#np-name')).toHaveValue('Local conflicting changes');
    await editor.getByRole('button', { name: 'Reload project…', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Keep editing', exact: true }).click();
    await expect(editor.locator('#np-name')).toHaveValue('Local conflicting changes');
    await editor.getByRole('button', { name: 'Reload project…', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Reload project', exact: true }).click();
    await expect(editor.locator('#np-name')).toHaveValue('Saved from another tab');
    assert.equal(draftWrites, draftWritesAfterCreate);
    console.log('PASS: concurrent edits reject a stale save, retain local input and require confirmation before reloading');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
