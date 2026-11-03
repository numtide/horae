// Run against an isolated, seeded dev-login instance with Playwright installed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/timesheet-header.cjs
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push({ message: error.message, stack: error.stack, url: page.url() }));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await page.locator('.ts-header .ts-saved').waitFor();
    // While a week reloads the total is not rendered; the view tabs must not
    // move into its place. Detaching the total reproduces that frame.
    // Includes widths where the header wraps beside the expanded rail (769) and
    // without it (520-650), not just the phone and desktop extremes.
    for (const width of [1440, 1024, 769, 650, 600, 520, 390, 320]) {
      await page.setViewportSize({ width, height: 1000 });
      const tabs = () => page.locator('.ts-header .segmented').evaluate(el => {
        const r = el.getBoundingClientRect();
        return { x: r.x, y: r.y, right: innerWidth - r.right };
      });
      const withTotal = await tabs();
      const total = await page.locator('.ts-header .ts-saved').evaluate(el => {
        const r = el.getBoundingClientRect();
        return { right: r.right, y: r.y };
      });
      const withoutTotal = await page.locator('.ts-header').evaluate(header => {
        const saved = header.querySelector('.ts-saved'), next = saved.nextSibling;
        saved.remove();
        const r = header.querySelector('.segmented').getBoundingClientRect();
        header.insertBefore(saved, next);
        return { x: r.x, y: r.y, right: innerWidth - r.right };
      });
      assert.deepEqual(withoutTotal, withTotal, JSON.stringify({ width }));
      // The total keeps its place beside the tabs.
      if (Math.abs(total.y - withTotal.y) < 8) assert.ok(withTotal.x - total.right <= 17, JSON.stringify({ width, total, withTotal }));
    }
    console.log('PASS: the timesheet view tabs keep their place without the week total');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error({ url: page.url(), errors });
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
