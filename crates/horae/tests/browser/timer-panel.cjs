// Run against an isolated, seeded dev-login instance with Playwright installed.
// HORAE_TEST_URL=http://127.0.0.1:8093 node crates/horae/tests/browser/timer-panel.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
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
    // A client-side read means the app has hydrated and the rail handles clicks.
    const ready = page.waitForResponse(r => r.url().includes('/api/list_people') && r.status() === 200);
    await page.goto(`${base}/people`);
    await (await ready).finished();
    const pager = page.locator('#app-main .ts-pager');
    await expect(pager).toBeVisible();

    const trigger = page.locator('#sidebar-timer-trigger');
    const panel = page.locator('#sidebar-timer-pop');
    const catalog = page.waitForResponse(r => r.url().includes('/api/list_tracking_projects'));
    await trigger.click();
    await catalog;
    await expect(panel.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    await expect(panel).toBeFocused();

    // The panel reaches past the rail over the page header. Every point of it
    // that overlaps the week pager or a filter trigger must hit the panel,
    // not the positioned page content later in the document.
    const hits = await panel.evaluate(el => {
      const box = el.getBoundingClientRect();
      const covered = [...document.querySelectorAll('#app-main .ts-pager, #app-main .ts-pager *, #app-main .filter-bar button, #app-main [popovertarget]')]
        .map(other => other.getBoundingClientRect())
        .map(r => ({
          left: Math.max(r.left, box.left), right: Math.min(r.right, box.right),
          top: Math.max(r.top, box.top), bottom: Math.min(r.bottom, box.bottom),
        }))
        .filter(r => r.right - r.left > 2 && r.bottom - r.top > 2);
      return covered.map(r => {
        const x = (r.left + r.right) / 2, y = (r.top + r.bottom) / 2;
        const top = document.elementFromPoint(x, y);
        return { x, y, inside: el.contains(top), hit: top && `${top.tagName}.${top.className}` };
      });
    });
    assert.ok(hits.length > 0, 'the panel should overlap the page header at 1440px');
    assert.ok(hits.every(hit => hit.inside), JSON.stringify(hits));

    // Stretched footer buttons centre their labels.
    const offsets = await panel.locator('.sidebar-timer-form-actions .btn').evaluateAll(buttons => buttons.map(button => {
      const range = document.createRange();
      range.selectNodeContents(button);
      const text = range.getBoundingClientRect(), box = button.getBoundingClientRect();
      return { label: button.textContent, offset: (text.left + text.right) / 2 - (box.left + box.right) / 2 };
    }));
    assert.equal(offsets.length, 2);
    assert.ok(offsets.every(found => Math.abs(found.offset) < 1), JSON.stringify(offsets));
    if (process.env.HORAE_BROWSER_ARTIFACTS) await page.screenshot({ path: `${process.env.HORAE_BROWSER_ARTIFACTS}/timer-panel.png`, animations: 'disabled' });
    console.log('PASS: the Start-timer panel paints over the page header and centres its footer labels');

    // A nested picker closes first, then the panel; focus returns to the trigger.
    await panel.locator('#timer-project').click();
    const options = page.locator('#timer-project-options');
    await expect(options).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(options).toBeHidden();
    await expect(panel).toBeVisible();
    await expect(panel.locator('#timer-project')).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(panel).toBeHidden();
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');
    await expect(trigger).toBeFocused();
    console.log('PASS: Escape closes the nested picker, then the panel, and focus returns to the trigger');

    // Light dismiss: a click on the page closes it.
    await trigger.click();
    await expect(panel).toBeVisible();
    await page.locator('#app-main h1').first().click();
    await expect(panel).toBeHidden();
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');
    console.log('PASS: a click beside the panel closes it');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error({ url: page.url(), errors });
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
