// The leave dialog over editors that are dialogs themselves: the timesheet's
// "Add a row" and the permission editor. Only run through run-design-checks.sh
// against its disposable seeded database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const native = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('dialog', async dialog => {
    native.push(`${dialog.type()}: ${dialog.message()}`);
    await dialog.dismiss();
  });
  const leave = page.locator('#leave-discard');
  // Reaches a screen from Clients through the rail, so that Back stays in
  // this document and returns to Clients.
  const visit = async (link, ready) => {
    await page.goto(`${base}/clients`);
    // Clicked before hydration, the rail link loads a new document, and Back
    // then leaves it without asking.
    await expect(page.locator(link)).toHaveAttribute('data-dioxus-id', /\d+/);
    await page.locator(link).click();
    await expect(ready).toBeVisible();
  };
  // Back from a dialog editor with unsaved changes: the leave dialog opens on
  // top, and Escape closes it alone, with focus back inside the editor.
  const stayOver = async (editor, title) => {
    const url = page.url();
    await page.evaluate(() => history.back());
    await expect(leave).toBeVisible();
    await expect(leave.getByRole('heading', { name: title, exact: true })).toBeVisible();
    await expect(leave.getByRole('button', { name: 'Keep editing', exact: true })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(leave).toBeHidden();
    await expect(editor).toBeVisible();
    assert.ok(await editor.evaluate(element => element.contains(document.activeElement)), 'focus returns inside the editor');
    await expect(page).toHaveURL(url);
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    // An open "Add a row" counts as unsaved timesheet work.
    const addRow = page.getByRole('dialog', { name: 'Add a row', exact: true });
    const trigger = page.getByRole('button', { name: '＋ Add row', exact: true });
    await visit('a.nav-item[href^="/timesheet"]', trigger);
    await trigger.click();
    await expect(addRow).toBeVisible();
    await stayOver(addRow, 'Discard unsaved timesheet changes?');
    await page.evaluate(() => history.back());
    await leave.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(/\/clients\??$/);
    assert.deepEqual(native, []);
    console.log('PASS: Back from an open Add a row asks over it; Escape keeps the row dialog, Discard leaves');

    // The permission editor is a dialog with its own discard confirmation.
    const editor = page.getByRole('dialog', { name: 'Edit permissions', exact: true });
    const actions = page.locator('tbody tr').filter({ has: page.getByText('Admin User', { exact: true }) })
      .getByRole('button', { name: /^Actions/ });
    await visit('a.nav-item[href^="/people"]', actions);
    await actions.click();
    await page.getByRole('menuitem', { name: 'Permissions…', exact: true }).click();
    await expect(editor.getByRole('button', { name: 'Review changes', exact: true })).toBeVisible();
    // An administrator's grants are not edited one by one: change the profile.
    await editor.locator('#person-permissions-profile').click();
    await editor.locator('#person-permissions-profile-options').getByRole('option', { name: 'Member', exact: true }).click();
    await expect(page.locator('[data-editor-kind="permissions"][data-editor-state="dirty"]')).toHaveCount(1);
    await stayOver(editor, 'Discard unsaved permission changes?');
    await page.evaluate(() => history.back());
    await leave.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(/\/clients\??$/);
    await expect(editor).toHaveCount(0);
    assert.deepEqual(native, []);
    console.log('PASS: Back from the permission editor asks over it; Escape keeps the editor, Discard leaves');

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
})().catch(error => {
  console.error(error);
  process.exit(1);
});
