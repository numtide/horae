// Run against an isolated, seeded dev-login instance with Playwright installed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/timesheet-header.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push({ message: error.message, stack: error.stack, url: page.url() }));
  const headerWidth = () => page.locator('.ts-header').evaluate(el => el.getBoundingClientRect().width);
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    // A client-side read means the app has hydrated and handles clicks.
    const ready = page.waitForResponse(r => r.url().includes('/api/load_timesheet_tracking') && r.status() === 200);
    await page.reload();
    await (await ready).finished();
    await page.locator('.ts-header .ts-saved').waitFor();

    // While a week reloads the total is not rendered; the view tabs must not
    // move into its place. Detaching the total reproduces that frame. The
    // widths include the expanded rail (769), the band where the header wraps
    // without it, and headers just either side of the 640px layout switch.
    const sizes = [1440, 1024, 769, 650, 600, 520, 390, 320].map(width => ({ width }));
    sizes.push({ header: 645 }, { header: 635 });
    for (const size of sizes) {
      let width = size.width;
      if (size.header) {
        // Padding varies by breakpoint, so step the viewport until the header fits.
        width = 600;
        for (let i = 0; i < 3; i++) {
          await page.setViewportSize({ width, height: 1000 });
          width += Math.round(size.header - await headerWidth());
        }
      }
      await page.setViewportSize({ width, height: 1000 });
      if (size.header) assert.equal(Math.round(await headerWidth()), size.header, JSON.stringify({ size, width }));
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
      assert.deepEqual(withoutTotal, withTotal, JSON.stringify({ size, width }));
      // The total keeps its place beside the tabs.
      if (Math.abs(total.y - withTotal.y) < 8) assert.ok(withTotal.x - total.right <= 17, JSON.stringify({ size, total, withTotal }));
    }
    console.log('PASS: the timesheet view tabs keep their place without the week total');

    // The header's size container must not trap the page's fixed overlays:
    // a click anywhere outside the open date picker, the rail included, closes it.
    await page.setViewportSize({ width: 1440, height: 1000 });
    const trigger = page.locator('.ts-pager-label');
    await trigger.click();
    const picker = page.locator('.dp-pop');
    await expect(picker).toBeVisible();
    const brand = await page.locator('.app-sidebar .sidebar-brand-name').boundingBox();
    await page.mouse.click(brand.x + brand.width / 2, brand.y + brand.height / 2);
    await expect(picker).toBeHidden();
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');
    console.log('PASS: a click on the sidebar closes the open date picker');

    // Day view's pager is as wide as Week's for the same week, so the picker
    // opens under it the same way and stepping through weekdays, or out of the
    // current week, never moves it.
    const pagerBox = async (view, day) => {
      const ready = page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
      await page.goto(`${base}/timesheet/${view}/${day}`);
      await (await ready).finished();
      // The brand fonts swap in late; measure the label in its final face.
      await page.evaluate(() => document.fonts.ready);
      return page.locator('.ts-pager').boundingBox();
    };
    const iso = d => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
    const weekPager = await pagerBox('week', iso(new Date()));
    // The picker bands the configured week, so its first day is the app's week start.
    await trigger.click();
    const start = new Date(await picker.locator('.dp-day.picked').first().getAttribute('aria-label'));
    // The eighth day is in the next week, where the label stops saying "This week".
    for (let i = 0; i < 8; i++) {
      const day = iso(new Date(start.getFullYear(), start.getMonth(), start.getDate() + i));
      const dayPager = await pagerBox('day', day);
      assert.equal(dayPager.width, weekPager.width, day);
      await trigger.click();
      const pop = await picker.boundingBox();
      assert.ok(pop.x + pop.width <= dayPager.x + dayPager.width, `${day}: the picker overhangs the pager`);
    }
    // The reserved week label is hidden, so it is not part of the trigger's name.
    await expect(trigger).not.toHaveAccessibleName(/week|–/i);
    console.log('PASS: the Day view pager matches the Week view pager');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error({ url: page.url(), errors });
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
