// Run against an isolated, seeded dev-login instance with Playwright installed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/sidebar-rail.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push({ message: error.message, stack: error.stack, url: page.url() }));
  const box = locator => locator.evaluate(el => {
    const r = el.getBoundingClientRect();
    return { width: r.width, height: r.height, cx: r.x + r.width / 2, cy: r.y + r.height / 2 };
  });
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    // A client-side read means the app has hydrated and the rail handles clicks.
    const ready = page.waitForResponse(r => r.url().includes('/api/list_tracking_projects') && r.status() === 200);
    await page.reload();
    await (await ready).finished();
    const sidebar = page.locator('.app-sidebar');
    // Named by aria-label, so it stays findable once the collapsed rail hides the label.
    const timer = sidebar.getByRole('button', { name: 'Start timer', exact: true });
    await expect(timer).toBeVisible();
    const expanded = await box(timer);

    await sidebar.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
    await expect(sidebar).toHaveCSS('width', '68px');
    // The idle button is the running timer's 44px chip, its icon centred, in
    // the same column as the brand mark and the collapse button.
    const chip = await box(timer);
    assert.deepEqual([chip.width, chip.height], [44, 44], JSON.stringify(chip));
    const icon = await box(timer.locator('.sidebar-timer-icon'));
    const mark = await box(sidebar.locator('.horae-mark'));
    const collapse = await box(sidebar.locator('.sidebar-collapse'));
    for (const other of [icon, mark, collapse]) assert.ok(Math.abs(other.cx - chip.cx) < 0.5, JSON.stringify({ chip, other }));
    assert.ok(Math.abs(icon.cy - chip.cy) < 0.5, JSON.stringify({ chip, icon }));

    // The toggle keeps one label in both states.
    await sidebar.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
    await expect(sidebar).toHaveCSS('width', '264px');
    assert.deepEqual(await box(timer), expanded);
    console.log('PASS: collapsed idle timer is a centred 44px chip; expanded timer unchanged');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error({ url: page.url(), errors });
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
