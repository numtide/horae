// Run against an isolated, seeded dev-login instance with Playwright installed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/mobile-navigation.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    const ready = page.waitForResponse(r => r.url().includes('/api/list_clients') && r.status() === 200);
    await page.goto(`${base}/clients`);
    await (await ready).finished();
    const sidebar = page.locator('.app-sidebar');
    const open = page.getByRole('button', { name: 'Open navigation', exact: true });
    await expect(sidebar).toBeHidden();
    await expect(open).toBeVisible();
    const content = await page.locator('main').boundingBox();
    assert.equal(content.x, 0);
    assert.equal(content.width, 390);
    assert.equal(await page.locator('main').evaluate(el => getComputedStyle(el).paddingLeft), '16px');
    await open.focus();
    await page.keyboard.press('Tab');
    await expect(page.getByRole('button', { name: 'Add Client', exact: true })).toBeFocused();
    await open.focus();
    await page.keyboard.press('Enter');
    await expect(open).toHaveCount(0);
    const close = page.getByRole('button', { name: 'Close navigation', exact: true });
    await expect(close).toHaveAttribute('aria-expanded', 'true');
    await expect(sidebar).toBeVisible();
    await expect(sidebar.getByRole('link', { name: 'Projects', exact: true })).toBeVisible();
    await expect(sidebar.getByRole('button', { name: 'Collapse sidebar', exact: true })).toBeHidden();
    await sidebar.getByRole('link', { name: 'Projects', exact: true }).focus();
    await page.keyboard.press('Escape');
    await expect(sidebar).toBeHidden();
    await expect(open).toBeFocused();
    await open.tap();
    if (process.env.HORAE_TEST_SCREENSHOT) await page.screenshot({ path: process.env.HORAE_TEST_SCREENSHOT, fullPage: true });
    await sidebar.getByRole('link', { name: 'Projects', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await expect(sidebar).toBeHidden();
    await expect(page.locator('main')).toBeFocused();
    console.log('PASS: mobile content width, keyboard toggle, Escape/focus restoration and navigation close');

    await page.setViewportSize({ width: 1440, height: 1000 });
    await expect(open).toBeHidden();
    await expect(sidebar).toBeVisible();
    assert.equal((await sidebar.boundingBox()).width, 264);
    await sidebar.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
    await expect(sidebar).toHaveCSS('width', '68px');
    await page.setViewportSize({ width: 390, height: 320 });
    await expect(sidebar).toBeHidden();
    await open.click();
    await expect(sidebar).toHaveCSS('width', '390px');
    await expect(sidebar.locator('.nav-item-label').filter({ hasText: 'Projects' })).toBeVisible();
    await sidebar.getByRole('button', { name: 'Start timer', exact: true }).click();
    const picker = sidebar.locator('.sidebar-timer-pop');
    await expect(picker).toBeVisible();
    await picker.getByRole('button', { name: 'Start timer', exact: true }).click();
    await picker.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(picker).toHaveCount(0);
    await sidebar.locator('.sidebar-footer').click();
    await sidebar.getByRole('link', { name: 'Settings', exact: true }).click();
    await expect(page).toHaveURL(`${base}/settings`);
    await expect(sidebar).toBeHidden();
    await page.setViewportSize({ width: 1440, height: 1000 });
    await expect(sidebar).toHaveCSS('width', '68px');
    console.log('PASS: short mobile viewport reaches account navigation and preserves desktop collapse state');
    await sidebar.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
    const handle = await page.locator('.sidebar-resize').boundingBox();
    await page.mouse.move(handle.x + handle.width / 2, 100);
    await page.mouse.down();
    await page.mouse.move(320, 100);
    await page.mouse.up();
    await expect(sidebar).toHaveCSS('width', '320px');
    for (const width of [320, 768, 769, 1440]) {
      await page.setViewportSize({ width, height: 844 });
      if (width <= 768) {
        await expect(sidebar).toBeHidden();
        await open.click();
        await expect(sidebar).toHaveCSS('width', `${width}px`);
        await close.click();
      } else {
        await expect(open).toBeHidden();
        await expect(sidebar).toHaveCSS('width', '320px');
      }
    }
    console.log('PASS: desktop drag width survives mobile and breakpoint transitions');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
