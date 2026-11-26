// Leaving the new-invoice screen. Only run through run-design-checks.sh
// against its disposable seeded database.
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
const acme = sql("SELECT id FROM clients WHERE name = 'Acme Corp'");
const prepare = `${base}/invoices/new/client/${acme}`;

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  // A browser may suppress its own dialogs and answer "stay" unseen, so the
  // screen must never rely on one to explain a held navigation.
  const native = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('dialog', async dialog => {
    native.push(`${dialog.type()}: ${dialog.message()}`);
    await dialog.dismiss();
  });
  const sidebar = page.locator('a.nav-item[href^="/clients"]');
  const po = page.getByLabel('PO number', { exact: true });
  const discard = page.getByRole('dialog', { name: 'Discard unsaved invoice changes?', exact: true });
  // Step 2 through the app, as a person reaches it, with its preview loaded.
  const open = async () => {
    await page.getByRole('link', { name: 'Invoices', exact: true }).click();
    await page.getByRole('link', { name: 'New invoice', exact: true }).click();
    await page.locator('#inv-client').click();
    await page.getByRole('option', { name: 'Acme Corp', exact: true }).click();
    const previewed = page.waitForResponse(response => response.url().includes('/api/prepare_invoice'));
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await expect(page).toHaveURL(prepare);
    await (await previewed).finished();
    await expect(po).toBeVisible();
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.waitForLoadState('networkidle');

    // The preview fills the values in, but nothing was changed: leave freely.
    await open();
    await sidebar.click();
    await expect(page).toHaveURL(/\/clients\??$/);
    await expect(discard).toHaveCount(0);
    assert.deepEqual(native, []);
    console.log('PASS: an untouched new invoice leaves through the sidebar without a prompt');

    // A change asks first: keeping it stays with the edit, discarding leaves.
    await open();
    await po.fill('Unsaved PO');
    await sidebar.click();
    await expect(discard).toBeVisible();
    await discard.getByRole('button', { name: 'Keep editing', exact: true }).click();
    await expect(discard).toBeHidden();
    await expect(page).toHaveURL(prepare);
    await expect(po).toHaveValue('Unsaved PO');
    await sidebar.click();
    await discard.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(/\/clients\??$/);
    await expect(page.getByRole('heading', { name: 'Clients', exact: true })).toBeVisible();
    assert.deepEqual(native, []);
    console.log('PASS: a changed new invoice asks in the app before leaving; keep stays, discard leaves');

    // A rejected back navigation leaves later navigation working.
    await open();
    await po.fill('Unsaved PO');
    await page.evaluate(() => history.back());
    await discard.getByRole('button', { name: 'Keep editing', exact: true }).click();
    await expect(page).toHaveURL(prepare);
    await expect(po).toHaveValue('Unsaved PO');
    await sidebar.click();
    await discard.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(/\/clients\??$/);
    // Discarding on a back navigation goes back after all.
    await open();
    await po.fill('Unsaved PO');
    await page.evaluate(() => history.back());
    await discard.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/invoices/new`);
    await expect(page.getByRole('button', { name: 'Continue', exact: true })).toBeVisible();
    assert.deepEqual(native, []);
    console.log('PASS: after a rejected back navigation, a sidebar link still asks and leaves');

    // Back is rejected, and before the guard's undo runs the save ends and the
    // app moves on (as after creating a draft). That navigation waits for the
    // undo, and Back and Forward keep reaching the router afterwards.
    await open();
    await po.fill('Unsaved PO');
    await page.evaluate(() => {
      const go = history.go.bind(history);
      history.go = delta => {
        history.go = go;
        go(delta);
        document.querySelector('[data-editor-state]').setAttribute('data-editor-leaving', '');
        queueMicrotask(() => document.querySelector('a.nav-item[href^="/clients"]').click());
      };
      history.back();
    });
    await expect(page).toHaveURL(/\/clients\??$/);
    await expect(page.getByRole('heading', { name: 'Clients', exact: true })).toBeVisible();
    await page.evaluate(() => history.back());
    await expect(page).toHaveURL(prepare);
    await expect(page.getByRole('heading', { name: 'New invoice', exact: true })).toBeVisible();
    await page.evaluate(() => history.forward());
    await expect(page).toHaveURL(/\/clients\??$/);
    await expect(page.getByRole('heading', { name: 'Clients', exact: true })).toBeVisible();
    // The dialog the rejected Back opened closed when the app moved on.
    await expect(discard).toHaveCount(0);
    assert.deepEqual(native, []);
    console.log('PASS: a navigation while a rejected Back is undone waits for it, and history keeps reaching the router');

    // The dialog by keyboard and pointer: it opens on Keep editing, Escape and
    // the backdrop keep editing, focus returns to the link, the scroll stays.
    await page.setViewportSize({ width: 1440, height: 500 });
    await open();
    await po.fill('Unsaved PO');
    await page.evaluate(() => scrollTo(0, document.documentElement.scrollHeight));
    const scrolled = await page.evaluate(() => scrollY);
    assert.ok(scrolled > 0, 'the form scrolls at this height');
    await sidebar.focus();
    await page.keyboard.press('Enter');
    await expect(discard.getByRole('button', { name: 'Keep editing', exact: true })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(discard).toBeHidden();
    await expect(sidebar).toBeFocused();
    assert.equal(await page.evaluate(() => scrollY), scrolled);
    await page.keyboard.press('Enter');
    await expect(discard).toBeVisible();
    await page.mouse.click(4, 4);
    await expect(discard).toBeHidden();
    await expect(page).toHaveURL(prepare);
    await expect(po).toHaveValue('Unsaved PO');
    assert.equal(await page.evaluate(() => scrollY), scrolled);
    await page.setViewportSize({ width: 1440, height: 900 });
    await sidebar.click();
    await discard.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(/\/clients\??$/);
    console.log('PASS: the leave dialog opens on Keep editing; Escape and the backdrop stay, keeping focus and scroll');

    // What counts as a change: projects and the period, and changing them back
    // is no change.
    const projects = page.getByRole('region', { name: 'Projects', exact: true }).getByRole('checkbox');
    const keepEditing = async () => {
      const url = page.url();
      await sidebar.click();
      await discard.getByRole('button', { name: 'Keep editing', exact: true }).click();
      await expect(discard).toBeHidden();
      await expect(page).toHaveURL(url);
    };
    const clean = page.locator('[data-editor-kind="invoice"][data-editor-state="clean"]');
    await open();
    await projects.nth(1).click();
    await keepEditing();
    await projects.nth(1).click();
    await expect(clean).toHaveCount(1);
    const preset = await page.locator('#inv-period [aria-current="true"]').textContent();
    const other = preset.trim() === 'Last month' ? 'This month' : 'Last month';
    const pick = async name => {
      const previewed = page.waitForResponse(response => response.url().includes('/api/prepare_invoice'));
      await page.locator('#inv-period-trigger').click();
      await page.getByRole('menuitem', { name, exact: true }).click();
      await (await previewed).finished();
    };
    await pick(other);
    await keepEditing();
    await pick(preset.trim());
    await expect(clean).toHaveCount(1);
    await sidebar.click();
    await expect(page).toHaveURL(/\/clients\??$/);
    assert.deepEqual(native, []);
    console.log('PASS: ticking a project or changing the period asks; changing it back does not');

    // A duplicate starts from another invoice's values, which are no change either.
    const source = sql(`SELECT id FROM invoices WHERE client_id = '${acme}' ORDER BY id LIMIT 1`);
    assert.ok(source, 'invoice-preparation leaves an Acme invoice to duplicate');
    const previewed = page.waitForResponse(response => response.url().includes('/api/prepare_invoice'));
    await page.goto(`${prepare}/from/${source}`);
    await (await previewed).finished();
    await expect(po).toBeVisible();
    await expect(clean).toHaveCount(1);
    await po.fill('Changed duplicate');
    await keepEditing();
    await page.locator('a.nav-item[href^="/invoices"]').click();
    await discard.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(/\/invoices\??$/);
    assert.deepEqual(native, []);
    console.log('PASS: a duplicate is unchanged until edited');

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
})().catch(error => {
  console.error(error);
  process.exit(1);
});
