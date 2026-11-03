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
    // Expanded, the icon sits beside the label and the pair is centred together.
    const pair = await timer.evaluate(el => {
      const icon = el.querySelector('.sidebar-timer-icon').getBoundingClientRect();
      const label = el.querySelector('.sidebar-timer-label').getBoundingClientRect();
      const button = el.getBoundingClientRect();
      return { gap: label.left - icon.right, offset: (icon.left + label.right) / 2 - (button.left + button.width / 2) };
    });
    assert.ok(Math.abs(pair.gap - 8) < 0.5 && Math.abs(pair.offset) < 0.5, JSON.stringify(pair));

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

    // Initials are trimmed to cap height and baseline, so the letters (not the
    // font's taller-than-deep line box) sit in the middle of the circle.
    const initials = async avatar => avatar.evaluate(el => {
      const circle = el.getBoundingClientRect(), text = el.querySelector('.avatar-initials').getBoundingClientRect();
      return {
        initials: el.textContent, size: circle.width, fontSize: parseFloat(getComputedStyle(el).fontSize),
        height: text.height, dy: text.y + text.height / 2 - (circle.y + circle.height / 2), dx: text.x + text.width / 2 - (circle.x + circle.width / 2),
      };
    });
    const checkInitials = found => {
      assert.ok(Math.abs(found.dx) < 1 && Math.abs(found.dy) < 1, JSON.stringify(found));
      // Untrimmed, line-height: 1 makes the box one em tall; caps are well short of that.
      assert.ok(found.height < found.fontSize * 0.85, JSON.stringify(found));
    };
    checkInitials(await initials(sidebar.locator('.sidebar-footer .avatar')));
    await page.goto(`${base}/components`);
    const gallery = page.locator('.avatar:not(.avatar-empty)').filter({ hasText: 'LE' });
    await expect(gallery).toHaveCount(3);
    const sizes = [];
    for (const avatar of await gallery.all()) {
      const found = await initials(avatar);
      checkInitials(found);
      sizes.push(found.size);
    }
    assert.deepEqual(sizes, [26, 34, 52]);
    console.log('PASS: avatar initials are centred on their cap height at every size');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error({ url: page.url(), errors });
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
