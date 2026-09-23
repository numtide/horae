// All writes belong to run-design-checks.sh's disposable database.
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

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage();
  const errors = [];
  const requests = [];
  let release;
  let hold = true;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await page.getByRole('link', { name: 'Projects', exact: true }).click();
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    const editor = page.locator('.np-page');
    const status = editor.locator('header').getByRole('status');
    await expect(editor.locator('#np-name')).toBeVisible();
    await expect(status).not.toHaveText('Saving draft…');
    const count = sql('SELECT count(*) FROM projects');

    const blocked = async (action, name, type = 'alert') => {
      const historyLength = await page.evaluate(() => history.length);
      const nextDialog = page.waitForEvent('dialog', { timeout: 5000 });
      const navigation = action();
      const dialog = await nextDialog;
      assert.equal(dialog.type(), type);
      if (type === 'beforeunload') await dialog.dismiss();
      else await dialog.accept();
      await navigation;
      await expect(page).toHaveURL(`${base}/projects/new`);
      await expect(editor.locator('#np-name')).toHaveValue(name);
      assert.equal(await page.evaluate(() => history.length), historyLength, 'Rejected navigation must not add history entries');
    };

    await page.route('**/api/save_project_draft*', async route => {
      requests.push(route.request().postData());
      if (hold) await new Promise(resolve => { release = resolve; });
      return route.continue();
    });
    await editor.locator('#np-name').fill('Pending creation draft');
    await expect.poll(() => !!release).toBe(true);
    await blocked(() => page.locator('a.nav-item[href="/clients"]').click(), 'Pending creation draft');
    await blocked(() => page.evaluate(() => history.back()), 'Pending creation draft');
    await blocked(() => page.evaluate(() => history.replaceState(history.state, '', '/clients')), 'Pending creation draft');
    await blocked(() => page.evaluate(() => location.reload()), 'Pending creation draft', 'beforeunload');
    await blocked(() => page.evaluate(() => location.assign('/clients')), 'Pending creation draft', 'beforeunload');
    await editor.locator('#np-name').fill('Newer edit during save');
    hold = false;
    release();
    await expect(status).toContainText('Draft saved at');
    assert.equal(requests.length, 2, 'Changes during the pending write are saved serially');
    await page.unroute('**/api/save_project_draft*');
    await page.locator('a.nav-item[href="/clients"]').click();
    await expect(page).toHaveURL(`${base}/clients`);
    await page.evaluate(() => history.back());
    await expect(editor.locator('#np-name')).toHaveValue('Newer edit during save');
    console.log('PASS: pending creation saves protect sidebar, Back, replacement, reload and full-document exits without losing newer edits');

    const failed = [];
    await page.route('**/api/save_project_draft*', async route => {
      failed.push(route.request().postData());
      await route.fetch();
      return route.abort('failed');
    });
    await editor.locator('#np-name').fill('Committed draft without acknowledgement');
    await expect(status).toHaveText('Changes need attention');
    await blocked(() => page.evaluate(() => history.forward()), 'Committed draft without acknowledgement');
    await blocked(() => page.locator('a.nav-item[href="/clients"]').click(), 'Committed draft without acknowledgement');
    await blocked(() => page.evaluate(() => location.reload()), 'Committed draft without acknowledgement', 'beforeunload');
    await editor.locator('#np-name').fill('Latest recovered draft');
    await page.unroute('**/api/save_project_draft*');
    await page.route('**/api/save_project_draft*', route => {
      failed.push(route.request().postData());
      return route.continue();
    });
    await editor.getByRole('button', { name: 'Retry request', exact: true }).click();
    await expect(status).toContainText('Draft saved at');
    assert.equal(failed.length, 3);
    assert.equal(failed[0], failed[1], 'Retry must acknowledge the exact committed snapshot before saving newer input');
    await page.unroute('**/api/save_project_draft*');
    await page.evaluate(() => history.forward());
    await expect(page).toHaveURL(`${base}/clients`);
    await page.evaluate(() => history.back());
    await expect(editor.locator('#np-name')).toHaveValue('Latest recovered draft');
    console.log('PASS: a lost acknowledgement blocks Forward and exits until the identical retry and newer edits are acknowledged');

    // Explicit Cancel flushes a pending write, without a second navigation prompt.
    release = undefined;
    hold = true;
    await page.route('**/api/save_project_draft*', async route => {
      if (hold) await new Promise(resolve => { release = resolve; });
      return route.continue();
    });
    await editor.locator('#np-name').fill('Cancel preserves this draft');
    await expect.poll(() => !!release).toBe(true);
    await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/new`);
    hold = false;
    release();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.unroute('**/api/save_project_draft*');
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(editor.locator('#np-name')).toHaveValue('Cancel preserves this draft');
    assert.equal(sql('SELECT count(*) FROM projects'), count, 'A creation draft is not an operational project');
    await editor.getByRole('button', { name: 'Discard draft', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Discard draft', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(editor.locator('#np-name')).toHaveValue('');
    await editor.locator('#np-client').click();
    await page.getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await editor.locator('#np-name').fill('Navigation guard creation fixture');
    const finalizations = [];
    await page.route('**/api/finalize_project_draft*', async route => {
      finalizations.push(route.request().postData());
      await route.fetch();
      return route.abort('failed');
    });
    await editor.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(status).toHaveText('Changes need attention');
    assert.equal(Number(sql('SELECT count(*) FROM projects')), Number(count) + 1);
    await blocked(() => page.locator('a.nav-item[href="/clients"]').click(), 'Navigation guard creation fixture');
    await blocked(() => page.evaluate(() => history.back()), 'Navigation guard creation fixture');
    await blocked(() => page.evaluate(() => location.reload()), 'Navigation guard creation fixture', 'beforeunload');
    await page.unroute('**/api/finalize_project_draft*');
    await page.route('**/api/finalize_project_draft*', route => {
      finalizations.push(route.request().postData());
      return route.continue();
    });
    await editor.getByRole('button', { name: 'Retry request', exact: true }).click();
    await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}$/);
    assert.equal(finalizations.length, 2);
    assert.equal(finalizations[0], finalizations[1]);
    assert.equal(Number(sql('SELECT count(*) FROM projects')), Number(count) + 1);
    assert.deepEqual(errors, []);
    console.log('PASS: explicit Cancel preserves the draft, confirmed discard clears it, and successful creation releases the guard');
  } finally {
    hold = false;
    release?.();
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
