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
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const task = JSON.parse(sql("SELECT row_to_json(t) FROM (SELECT t.id, t.default_rate_cents, o.default_currency FROM tasks t JOIN organizations o ON o.id = t.org_id WHERE t.name = 'Development' AND o.name = 'Demo Org') t"));
assert.equal(task.default_currency, 'EUR');
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
  const createProject = async (mode, currency = 'USD') => {
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
    await expect(screen.locator('header').getByRole('status')).toContainText('Draft saved at');
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}$/);
    await expect(page.getByRole('region', { name: 'Project details', exact: true })).toContainText(`Task rate browser ${mode} ${currency}`);
    await readsFinished();
    const id = new URL(page.url()).pathname.split('/').pop();
    assert.match(id, /^[0-9a-f-]{36}$/);
    return id;
  };
  const storedRate = projectId => sql(`SELECT rate_cents FROM project_tasks WHERE project_id = '${projectId}' AND task_id = '${task.id}'`);
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.waitForLoadState('networkidle');
    for (const [width, amount, cents] of [[390, '80.25', '8025'], [768, '0', '0'], [1440, '123.45', '12345']]) {
      await page.setViewportSize({ width, height: 900 });
      const id = await createProject('Task');
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
    }
    for (const mode of ['Person', 'Project', 'fixed', 'nonbillable']) {
      await createProject(mode);
      await expect(page.locator('#project-task-rate')).toHaveCount(0);
      await expect(page.getByLabel('Enable an existing task', { exact: true })).toBeVisible();
    }
    const id = await createProject('Task', 'EUR');
    await page.getByLabel('Enable an existing task', { exact: true }).selectOption(task.id);
    await page.getByRole('button', { name: 'Enable task', exact: true }).click();
    await expect(page.getByLabel('Enable an existing task', { exact: true })).toHaveValue('');
    assert.equal(storedRate(id), String(task.default_rate_cents));
    await readsFinished();
    assert.deepEqual(errors, []);
    console.log('PASS: New Project task-rate modes reach detail; cross-currency rejection, exact/zero recovery, failed requests, keyboard and three viewport widths preserve values and stored rates');
  } catch (error) {
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot() });
    throw error;
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
