// Apps in administration (spec 024, DESIGN.md: Apps): the list with Horae's
// own app first, the notice while the public address is unset, registering a
// server app whose secret shows once, a refused address under its field, the
// app page's Actions menu, and revoking an authorization through its
// confirmation. With HORAE_BROWSER_ARTIFACTS it saves every state at 1280 and
// 390px, dark and light. HORAE_APPS_CAN_CONNECT=1 says the server under test
// has a public address, so the notice must be absent.
// It registers and deletes its own app, and inserts one authorization of the
// administrator for it.
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { expect } = playwright;
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const { join } = require('node:path');
const evidence = process.env.HORAE_BROWSER_ARTIFACTS;
if (evidence) mkdirSync(evidence, { recursive: true });
const canConnect = process.env.HORAE_APPS_CAN_CONNECT === '1';
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();

(async () => {
  const browser = await playwright.chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 }, colorScheme: 'dark' });
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const settled = async () => {
    await page.waitForLoadState('networkidle');
    await expect(page.locator('main [role="status"].loading, main .table-skeleton')).toHaveCount(0);
  };
  const open = async path => {
    await page.goto(`${base}${path}`);
    await expect(page.locator('.page-header').first()).toBeVisible();
    await settled();
  };
  // Each state at both widths and in both themes; the page is left as found.
  const shoot = async name => {
    if (!evidence) return;
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
      for (const theme of ['dark', 'light']) {
        await page.evaluate(value => { document.documentElement.dataset.theme = value; }, theme);
        await page.screenshot({ path: join(evidence, `${name}-${width}-${theme}.png`), fullPage: true, animations: 'disabled' });
      }
    }
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
  };
  const main = page.locator('.adm-main');
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    // Workspace › Apps, after API tokens.
    await open('/admin/api-tokens');
    const subnav = page.locator('.adm-nav');
    const links = await subnav.getByRole('link').allTextContents();
    assert.equal(links[links.indexOf('API tokens') + 1], 'Apps', 'Apps follows API tokens');
    await subnav.getByRole('link', { name: 'Apps', exact: true }).click();
    await page.waitForURL(`${base}/admin/apps`);
    await settled();
    await expect(subnav.getByRole('link', { name: 'Apps', exact: true })).toHaveAttribute('aria-current', 'page');

    // Horae's own app comes first, its access in words, never scope codes.
    const table = main.getByRole('table', { name: 'Apps' });
    const first = table.locator('tbody tr').first();
    await expect(first).toContainText('Horae for Android');
    await expect(first).toContainText('Horae’s own app');
    await expect(first).toContainText('Time, projects');
    await expect(main).not.toContainText('time:read');
    await expect(main.locator('.table-container')).toHaveCSS('border-radius', '16px');
    const notice = main.locator('#apps-not-ready');
    if (canConnect) {
      await expect(notice).toHaveCount(0);
      await shoot('apps-list');
    } else {
      await expect(notice).toContainText('Apps can’t connect yet');
      await expect(notice).not.toContainText('HORAE_');
      await shoot('apps-not-ready');
    }

    // The built-in app's menu has Open and Enable only.
    await first.getByRole('button', { name: 'Actions for Horae for Android' }).click();
    const builtInMenu = page.getByRole('menu').filter({ visible: true });
    await expect(builtInMenu.getByRole('menuitem')).toHaveText(['Open', 'Enable']);
    await shoot('apps-menu-built-in');
    await page.keyboard.press('Escape');

    // Register a server app. A refused address shows under its field and the
    // form keeps what was typed.
    await main.getByRole('button', { name: 'Register app' }).click();
    const editor = page.getByRole('dialog', { name: 'Register app' });
    await expect(editor).toBeVisible();
    await editor.getByLabel('Name', { exact: true }).fill('Invoice sync');
    await editor.getByRole('radio', { name: /Runs on a server/ }).click();
    await editor.locator('#app-redirects').fill('https://sync.example.com/callback#done');
    await shoot('apps-register');
    await editor.getByRole('button', { name: 'Register app' }).click();
    await expect(editor.locator('#app-redirects-error')).toBeVisible();
    await expect(editor.locator('#app-redirects')).toHaveAttribute('aria-invalid', 'true');
    await expect(editor.getByLabel('Name', { exact: true })).toHaveValue('Invoice sync');
    await shoot('apps-register-error');
    await editor.locator('#app-redirects').fill('https://sync.example.com/callback\nhttps://staging.sync.example.com/callback');
    await editor.getByRole('button', { name: 'Register app' }).click();

    // The secret shows once, with the client id and one button.
    const reveal = page.getByRole('dialog', { name: 'Invoice sync is registered' });
    await expect(reveal).toBeVisible();
    const secret = await reveal.locator('#app-secret-value').inputValue();
    assert.match(secret, /^horae_ocs_/);
    await expect(reveal.getByRole('button')).toHaveText(['Copy', 'I’ve stored it']);
    await shoot('apps-secret');
    await reveal.getByRole('button', { name: 'I’ve stored it' }).click();

    // Its page: details, access in sentences, the Actions menu.
    await page.waitForURL(/\/admin\/apps\/[0-9a-f-]{36}$/);
    await settled();
    const appId = page.url().split('/').pop();
    await expect(page.getByRole('heading', { name: 'Invoice sync', level: 1 })).toBeVisible();
    await expect(main).toContainText('Runs on a server (has a secret)');
    await expect(main).toContainText('https://staging.sync.example.com/callback');
    await expect(main).toContainText('See the clients, projects and tasks you work on');
    await expect(main).not.toContainText(secret);
    const clientId = await main.locator('#app-client-id').inputValue();
    assert.match(clientId, /^horae_app_/);

    // Someone has allowed it: the administrator, for the test.
    sql(`INSERT INTO oauth_grants (id, org_id, client_id, user_id, scopes, expires_at)
         SELECT gen_random_uuid(), c.org_id, c.id, u.id, c.scopes, now() + interval '1 year'
         FROM oauth_clients c JOIN users u ON u.org_id = c.org_id AND u.email = 'admin@example.com'
         WHERE c.id = '${appId}'`);
    await open(`/admin/apps/${appId}`);
    const grants = main.getByRole('table', { name: 'Authorizations' });
    await expect(grants.locator('tbody tr')).toHaveCount(1);
    await expect(grants).toContainText('admin@example.com');
    await expect(main).toContainText('1 active');
    await shoot('apps-detail');

    await page.getByRole('button', { name: 'Actions', exact: true }).click();
    const actions = page.getByRole('menu').filter({ visible: true });
    await expect(actions.getByRole('menuitem')).toHaveText(['Edit…', 'Replace secret…', 'Disable', 'Delete…']);
    await expect(actions.getByRole('menuitem', { name: 'Delete…' })).toHaveClass(/danger/);
    await shoot('apps-detail-menu');
    await actions.getByRole('menuitem', { name: 'Disable' }).click();
    await expect(page.locator('.page-header .badge')).toHaveText('Disabled');

    // Revoke sits in the row's menu and asks first.
    await grants.getByRole('button', { name: /^Actions for / }).click();
    await page.getByRole('menu').filter({ visible: true }).getByRole('menuitem', { name: 'Revoke…' }).click();
    const confirm = page.getByRole('dialog', { name: /access\?$/ });
    await expect(confirm).toBeVisible();
    await expect(confirm.locator('#grant-revoke-dialog-confirm')).toHaveClass(/btn-danger-solid/);
    await shoot('apps-revoke');
    await confirm.locator('#grant-revoke-dialog-confirm').click();
    await expect(confirm).toBeHidden();
    await expect(grants).toContainText('Nobody has allowed this app yet');
    await expect(main).toContainText('0 active');

    // Delete asks first, then the list no longer has it.
    await page.getByRole('button', { name: 'Actions', exact: true }).click();
    await page.getByRole('menu').filter({ visible: true }).getByRole('menuitem', { name: 'Delete…' }).click();
    const deletion = page.getByRole('dialog', { name: 'Delete Invoice sync?' });
    await expect(deletion).toBeVisible();
    await deletion.getByRole('button', { name: 'Delete app' }).click();
    await page.waitForURL(`${base}/admin/apps`);
    await settled();
    await expect(main.getByRole('table', { name: 'Apps' })).not.toContainText('Invoice sync');

    assert.deepEqual(errors, []);
    console.log('PASS: apps list, register with a one-time secret, app page, revoke and delete');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
