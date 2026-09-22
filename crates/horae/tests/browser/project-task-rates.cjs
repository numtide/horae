// Only the design runner's disposable database may be inspected by this fixture.
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
const task = JSON.parse(sql("SELECT row_to_json(t) FROM (SELECT t.id, t.default_rate_cents, t.default_rate_currency, o.default_currency FROM tasks t JOIN organizations o ON o.id = t.org_id WHERE t.name = 'Development' AND o.name = 'Demo Org') t"));
assert.equal(task.default_currency, 'EUR');
assert.equal(task.default_rate_currency, 'EUR');
assert.ok(task.default_rate_cents > 0);

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
  const createProject = async (mode, currency = 'USD', budgetScope, catalog) => {
    await readsFinished();
    await page.goto(`${base}/projects/new`);
    const screen = page.locator('.np-page');
    await screen.getByRole('button', { name: 'Client', exact: true }).click();
    await page.getByRole('dialog', { name: 'Choose Client', exact: true }).getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await screen.getByLabel('Project name', { exact: true }).fill(`Task rate browser ${mode} ${currency}`);
    await screen.getByRole('button', { name: 'Currency', exact: true }).click();
    await page.getByRole('listbox', { name: 'Choose Currency', exact: true }).getByRole('option', { name: currency, exact: true }).click();
    if (mode === 'fixed') {
      await screen.getByRole('radio', { name: /^Fixed Fee/ }).check();
      await screen.getByLabel(`Fee (${currency})`, { exact: true }).fill('500');
    } else if (mode === 'nonbillable') {
      await screen.getByRole('radio', { name: /^Non-Billable/ }).check();
    } else {
      await screen.getByRole('radio', { name: new RegExp(`^${mode} hourly rate`) }).check();
      if (mode === 'Project') await screen.getByLabel(`Hourly rate (${currency})`, { exact: true }).fill('90');
    }
    if (budgetScope) {
      await screen.locator('#np-budget-mode').click();
      await page.getByRole('listbox', { name: 'Choose Budget', exact: true }).getByRole('option', { name: `Hours per ${budgetScope}`, exact: true }).click();
    }
    if (catalog) {
      await screen.getByRole('button', { name: 'Development', exact: true }).click();
      const rate = screen.getByRole('textbox', { name: `Hourly rate for Development (${currency})`, exact: true });
      await expect(rate).toHaveAttribute('placeholder', catalog.placeholder);
      if (catalog.override !== undefined) {
        await screen.getByRole('button', { name: 'Save project', exact: true }).click();
        await expect(screen.getByRole('alert')).toContainText('currency is unknown or incompatible');
        await expect(rate).toBeFocused();
        await expect(rate).toHaveValue('');
        await expect(rate).toHaveAttribute('aria-invalid', 'true');
        await rate.fill(catalog.override);
      }
    }
    await expect(screen.locator('header').getByRole('status')).toContainText(
      catalog?.override !== undefined ? 'Changes need attention' : 'Draft saved at'
    );
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}$/);
    await expect(page.getByRole('region', { name: 'Project details', exact: true })).toContainText(`Task rate browser ${mode} ${currency}`);
    await readsFinished();
    const id = new URL(page.url()).pathname.split('/').pop();
    assert.match(id, /^[0-9a-f-]{36}$/);
    return id;
  };
  const storedRate = projectId => sql(`SELECT rate_cents FROM project_tasks WHERE project_id = '${projectId}' AND task_id = '${task.id}'`);
  const selectEditor = async id => {
    const row = page.locator('.proj-row').filter({ has: page.locator(`a[href="/projects/${id}"]`) });
    await row.getByRole('button', { name: 'Actions' }).click();
    await row.getByRole('menuitem', { name: 'Edit', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Edit Project', exact: true })).toBeVisible();
    return row;
  };
  const openEditor = async id => {
    await readsFinished();
    await page.goto(`${base}/projects`);
    return selectEditor(id);
  };
  const verifyEditor = async (id, mode, pendingRead = false) => {
    let releaseRead;
    if (pendingRead) {
      const blocked = new Promise(resolve => { releaseRead = resolve; });
      await page.route('**/api/get_project_edit_policy*', async route => { await blocked; await route.abort(); });
    }
    try {
      await openEditor(id);
      if (pendingRead) {
        await expect(page.getByText('Loading project editing options…', { exact: true })).toBeVisible();
        await expect(page.locator('#proj-name')).toHaveCount(0);
        await expect(page.getByRole('button', { name: 'Save Changes', exact: true })).toHaveCount(0);
      }
    } finally { if (releaseRead) releaseRead(); }
    if (pendingRead) {
      await expect(page.getByRole('alert')).toContainText('Could not load project editing options');
      await expect(page.locator('#proj-name')).toHaveCount(0);
      await readsFinished();
      await page.unroute('**/api/get_project_edit_policy*');
      await page.getByRole('button', { name: 'Retry editing options', exact: true }).click();
    }
    await expect(page.getByLabel('Type', { exact: true })).toBeDisabled();
    await expect(page.getByLabel('Currency', { exact: true })).toBeDisabled();
    const rate = page.getByLabel('Hourly rate', { exact: true });
    if (mode === 'Project') {
      await expect(rate).toBeEnabled();
      await rate.fill('');
      await page.getByRole('button', { name: 'Save Changes', exact: true }).click();
      await expect(page.locator('.card .alert-danger')).toContainText('Project rate is required');
      await rate.fill('0');
    } else {
      await expect(rate).toBeDisabled();
    }
    const monetary = !['fixed', 'nonbillable'].includes(mode);
    await expect(page.locator('#proj-budget option[value="amount"]')).toHaveCount(monetary ? 1 : 0);
    await page.getByLabel('Budget', { exact: true }).selectOption('hours');
    await page.getByLabel('Budget hours', { exact: true }).fill('3:30');
    const name = `Edited ${id}`;
    await page.getByLabel('Name', { exact: true }).fill(name);
    if (process.env.HORAE_TEST_SCREENSHOT_DIR) await page.locator('.card').filter({ has: page.getByRole('heading', { name: 'Edit Project', exact: true }) }).screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/project-editor-${mode}-${page.viewportSize().width}.png` });
    let releaseSave;
    const blocked = new Promise(resolve => { releaseSave = resolve; });
    await page.route('**/api/update_project*', async route => { await blocked; await route.abort(); });
    try {
      await page.getByRole('button', { name: 'Save Changes', exact: true }).click();
      await expect(page.getByLabel('Name', { exact: true })).toBeDisabled();
      await expect(page.getByLabel('Budget hours', { exact: true })).toBeDisabled();
      await expect(page.getByRole('button', { name: 'Save Changes', exact: true })).toBeDisabled();
      await expect(page.locator('.page-header').getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    } finally { releaseSave(); }
    await expect(page.locator('.card .alert-danger')).toBeVisible();
    await expect(page.getByLabel('Name', { exact: true })).toBeEnabled();
    await expect(page.getByLabel('Name', { exact: true })).toHaveValue(name);
    await expect(page.getByLabel('Budget hours', { exact: true })).toHaveValue('3:30');
    if (mode === 'Project') await expect(rate).toHaveValue('0');
    await readsFinished();
    await page.unroute('**/api/update_project*');
    await page.getByRole('button', { name: 'Save Changes', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { name: 'Edit Project', exact: true })).toHaveCount(0);
    assert.equal(sql(`SELECT name FROM projects WHERE id = '${id}'`), name);
    assert.equal(sql(`SELECT budget_minutes FROM projects WHERE id = '${id}'`), '210');
    if (mode === 'Project') assert.equal(sql(`SELECT rate_cents FROM projects WHERE id = '${id}'`), '0');
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
    await readsFinished();
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.waitForLoadState('networkidle');
    let firstConfiguredId;
    for (const [width, amount, cents] of [[390, '80.25', '8025'], [768, '0', '0'], [1440, '123.45', '12345']]) {
      await page.setViewportSize({ width, height: 900 });
      const id = await createProject('Task');
      firstConfiguredId ??= id;
      const rate = page.getByLabel('Task hourly rate (USD)', { exact: true });
      await expect(rate).toBeVisible();
      await page.getByLabel('Enable an existing task', { exact: true }).selectOption(task.id);
      const enable = page.getByRole('button', { name: 'Enable task', exact: true });
      await enable.click();
      await expect(page.getByRole('alert')).toContainText('explicit rate in the project currency');
      assert.equal(storedRate(id), '');
      await rate.fill('-1');
      await enable.click();
      await expect(page.getByRole('alert')).toContainText('Rate cannot be negative');
      await expect(rate).toHaveValue('-1');
      assert.equal(storedRate(id), '');
      await rate.fill(amount);
      let release;
      const blocked = new Promise(resolve => { release = resolve; });
      await page.route('**/api/link_project_task*', async route => { await blocked; await route.abort(); });
      try {
        await enable.click();
        await expect(rate).toBeDisabled();
        await expect(page.getByLabel('Enable an existing task', { exact: true })).toBeDisabled();
        await expect(enable).toBeDisabled();
      } finally {
        release();
      }
      await expect(page.getByRole('alert')).toBeVisible();
      await expect(rate).toBeEnabled();
      await expect(rate).toHaveValue(amount);
      await readsFinished();
      await page.unroute('**/api/link_project_task*');
      await enable.focus();
      await page.keyboard.press('Enter');
      await expect(page.getByRole('alert')).toHaveCount(0);
      await expect(rate).toHaveValue('');
      assert.equal(storedRate(id), cents);
      await expect(page.getByLabel('Enable an existing task', { exact: true })).toHaveValue('');
      await expect(page.locator(`#project-task option[value="${task.id}"]`)).toBeDisabled();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/project-task-rate-${width}.png` });
      await verifyEditor(id, 'Task', width === 390);
    }
    for (const mode of ['Person', 'Project', 'fixed', 'nonbillable']) {
      const id = await createProject(mode);
      await expect(page.locator('#project-task-rate')).toHaveCount(0);
      await expect(page.getByLabel('Enable an existing task', { exact: true })).toBeVisible();
      await verifyEditor(id, mode);
    }
    for (const scope of ['task', 'person']) {
      const id = await createProject(scope === 'task' ? 'Task' : 'Person', 'EUR', scope);
      await openEditor(id);
      await expect(page.getByLabel('Budget', { exact: true })).toBeDisabled();
      await expect(page.getByLabel('Budget hours', { exact: true })).toBeDisabled();
      await page.getByLabel('Name', { exact: true }).fill(`Scoped ${scope}`);
      await page.getByRole('button', { name: 'Save Changes', exact: true }).click();
      await expect(page.getByRole('heading', { name: 'Edit Project', exact: true })).toHaveCount(0);
      assert.equal(sql(`SELECT budget_scope FROM project_settings WHERE project_id = '${id}'`), scope);
      assert.equal(sql(`SELECT budget_minutes FROM projects WHERE id = '${id}'`), '');
    }
    // Invoice preparation configures the seeded projects. Give this legacy
    // assertion its own project instead of depending on another suite's state.
    const legacyId = '01960000-0000-7000-8000-000000000001';
    assert.equal(sql(`INSERT INTO projects (id, org_id, client_id, name, currency)
      SELECT '${legacyId}', c.org_id, c.id, 'Legacy editor fixture', c.currency
      FROM clients c JOIN tasks t ON t.org_id = c.org_id
      WHERE t.id = '${task.id}' AND c.active ORDER BY c.id LIMIT 1 RETURNING id`), legacyId);
    await openEditor(legacyId);
    for (const label of ['Name', 'Type', 'Currency', 'Hourly rate', 'Budget']) await expect(page.getByLabel(label, { exact: true })).toBeEnabled();
    await expect(page.getByText("Leave blank to use the user's default. Task and assignment overrides take priority. Zero is a free rate.", { exact: true })).toBeVisible();
    let releaseSwitch;
    const switching = new Promise(resolve => { releaseSwitch = resolve; });
    await page.route('**/api/get_project_edit_policy*', async route => { await switching; await route.continue(); });
    try {
      await selectEditor(firstConfiguredId);
      await expect(page.getByText('Loading project editing options…', { exact: true })).toBeVisible();
      await expect(page.locator('#proj-type')).toHaveCount(0);
      await expect(page.getByRole('button', { name: 'Save Changes', exact: true })).toHaveCount(0);
    } finally { releaseSwitch(); }
    await expect(page.getByLabel('Type', { exact: true })).toBeDisabled();
    await readsFinished();
    await page.unroute('**/api/get_project_edit_policy*');
    await page.locator('.page-header').getByRole('button', { name: 'Cancel', exact: true }).click();
    const id = await createProject('Task', 'EUR');
    await page.getByLabel('Enable an existing task', { exact: true }).selectOption(task.id);
    await page.getByRole('button', { name: 'Enable task', exact: true }).click();
    await expect(page.getByLabel('Enable an existing task', { exact: true })).toHaveValue('');
    assert.equal(storedRate(id), String(task.default_rate_cents));
    for (const [source, amount, currency, placeholder, override, expected] of [
      ['USD', 8000, 'USD', '80.00', undefined, '8000'],
      [null, 0, 'EUR', 'Enter rate', '0', '0'],
      ['USD', 8000, 'EUR', 'Enter rate', '8.25', '825'],
    ]) {
      sql(`UPDATE tasks SET default_rate_cents = ${amount}, default_rate_currency = ${source === null ? 'NULL' : `'${source}'`} WHERE id = '${task.id}'`);
      const project = await createProject('Task', currency, undefined, { placeholder, override });
      assert.equal(storedRate(project), expected);
    }
    sql(`UPDATE tasks SET default_rate_cents = ${task.default_rate_cents}, default_rate_currency = '${task.default_rate_currency}' WHERE id = '${task.id}'`);
    await readsFinished();
    assert.deepEqual(errors, []);
    console.log('PASS: New Project task rates and editor capabilities preserve configured/legacy modes, source currencies, unknown/zero recovery, scoped budgets and keyboard recovery across three widths');
  } catch (error) {
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot() });
    throw error;
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
