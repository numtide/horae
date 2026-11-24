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
  const sidebar = page.locator('a.nav-item[href="/clients"]');
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
    await expect(page).toHaveURL(`${base}/clients`);
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
    await expect(page).toHaveURL(`${base}/clients`);
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
    await expect(page).toHaveURL(`${base}/clients`);
    // Discarding on a back navigation goes back after all.
    await open();
    await po.fill('Unsaved PO');
    await page.evaluate(() => history.back());
    await discard.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/invoices/new`);
    await expect(page.getByRole('button', { name: 'Continue', exact: true })).toBeVisible();
    assert.deepEqual(native, []);
    console.log('PASS: after a rejected back navigation, a sidebar link still asks and leaves');

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
})().catch(error => {
  console.error(error);
  process.exit(1);
});
