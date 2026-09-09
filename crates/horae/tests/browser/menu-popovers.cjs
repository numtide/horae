// Run against an isolated, seeded DEV_LOGIN instance; no business data is changed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/menu-popovers.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 320, height: 1000 }, hasTouch: true });
  const errors = [], failures = [];
  page.on('pageerror', error => errors.push(error.message));
  async function visit(path, resource) {
    const ready = page.waitForResponse(r => r.url().includes(`/api/${resource}`) && r.status() === 200);
    await page.goto(`${base}${path}`);
    await (await ready).finished();
  }
  async function check(name, run) {
    try { await run(); console.log(`PASS: ${name}`); }
    catch (error) { failures.push(name); console.error(`FAIL: ${name}: ${error.message}`); }
  }
  async function visibleItems(menu) {
    // Inspect before locator.click/focus can auto-scroll and conceal a clipped menu.
    await expect(menu).toBeVisible();
    return menu.evaluate(menu => {
      const box = menu.getBoundingClientRect();
      const items = [...menu.querySelectorAll('button')].map(item => {
        const r = item.getBoundingClientRect(), x = r.x + r.width / 2, y = r.y + r.height / 2;
        return { text: item.textContent, x, y, hit: item.contains(document.elementFromPoint(x, y)) };
      });
      return { inside: box.x >= 0 && box.y >= 0 && box.right <= innerWidth && box.bottom <= innerHeight, items };
    });
  }
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await check('first and last project-row actions are initially visible and clickable', async () => {
      for (const [width, height] of [[320, 1000], [390, 320], [1440, 800]]) {
        await page.setViewportSize({ width, height });
        await visit('/projects', 'list_projects');
        await expect(page.locator('.proj-row').first()).toBeVisible();
        for (const row of [page.locator('.proj-row').first(), page.locator('.proj-row').last()]) {
          await row.getByRole('button', { name: 'Actions' }).click();
          const menu = row.getByRole('menu');
          const geometry = await visibleItems(menu);
          assert.ok(geometry.inside && geometry.items.every(item => item.hit), JSON.stringify({ width, height, geometry }));
          if (process.env.HORAE_TEST_SCREENSHOT && width === 320)
            await page.screenshot({ path: process.env.HORAE_TEST_SCREENSHOT });
          const edit = geometry.items.find(item => item.text === 'Edit');
          await page.mouse.click(edit.x, edit.y);
          await expect(page.getByRole('heading', { name: 'Edit Project', exact: true })).toBeVisible();
          await expect(menu).toBeHidden();
          await page.locator('.page-header').getByRole('button', { name: 'Cancel', exact: true }).click();
        }
      }
    });
    await check('menu keyboard, disabled items, outside click and repeated opening', async () => {
      await page.setViewportSize({ width: 1280, height: 800 });
      await visit('/components', 'get_me');
      const trigger = page.getByRole('button', { name: /^Actions/ });
      await trigger.scrollIntoViewIfNeeded();
      await trigger.focus();
      await page.keyboard.press('Enter');
      const menu = page.getByRole('menu');
      await expect(menu.getByRole('menuitem', { name: 'Edit', exact: true })).toBeFocused();
      await page.keyboard.press('End');
      await expect(menu.getByRole('menuitem', { name: 'Delete', exact: true })).toBeFocused();
      await page.keyboard.press('ArrowDown');
      await expect(menu.getByRole('menuitem', { name: 'Edit', exact: true })).toBeFocused();
      await page.keyboard.press('p');
      await expect(menu.getByRole('menuitem', { name: 'Pin', exact: true })).toBeFocused();
      await page.keyboard.press('Escape');
      await expect(menu).toBeHidden();
      await expect(trigger).toBeFocused();
      await expect(trigger).toHaveAttribute('aria-expanded', 'false');
      await page.keyboard.press('ArrowUp');
      await expect(menu.getByRole('menuitem', { name: 'Delete', exact: true })).toBeFocused();
      await page.keyboard.press('Home');
      await expect(menu.getByRole('menuitem', { name: 'Edit', exact: true })).toBeFocused();
      await page.keyboard.press('Tab');
      await expect(menu).toBeHidden();
      assert.equal(await trigger.evaluate(el => el === document.activeElement), false);
      for (let i = 0; i < 3; i++) {
        await trigger.tap();
        await expect(menu).toBeVisible();
        await expect(trigger).toHaveAttribute('aria-expanded', 'true');
        await trigger.tap();
        await expect(menu).toBeHidden();
        await trigger.tap();
        await expect(menu).toBeVisible();
        const disabled = menu.getByRole('menuitem', { name: 'Unavailable', exact: true });
        await expect(disabled).toBeDisabled();
        const disabledBox = await disabled.boundingBox();
        await page.mouse.click(disabledBox.x + disabledBox.width / 2, disabledBox.y + disabledBox.height / 2);
        await expect(menu).toBeVisible();
        await page.mouse.click(2, 2);
        await expect(menu).toBeHidden();
      }
    });
    await check('scroll and resize dismiss without leaving stale menus or focus', async () => {
      await page.setViewportSize({ width: 390, height: 600 });
      await visit('/projects', 'list_projects');
      const row = page.locator('.proj-row').last();
      const trigger = row.getByRole('button', { name: 'Actions' });
      const menu = row.getByRole('menu');
      await trigger.click();
      await expect(menu.getByRole('menuitem', { name: 'Edit', exact: true })).toBeFocused();
      await page.keyboard.press('ArrowDown');
      await expect(menu.getByRole('menuitem', { name: 'Archive', exact: true })).toBeFocused();
      await page.locator('.proj-scroll').evaluate(el => { el.scrollLeft -= 40; });
      await expect(menu).toBeHidden();
      await expect(trigger).toHaveAttribute('aria-expanded', 'false');
      await trigger.click();
      await expect(menu).toBeVisible();
      await page.setViewportSize({ width: 320, height: 600 });
      await expect(menu).toBeHidden();
      await expect(trigger).toBeFocused();
      await trigger.click();
      await expect(menu).toBeVisible();
      await page.evaluate(() => window.scrollBy(0, -30));
      await expect(menu).toBeHidden();
    });
    await check('short-screen menu scrolls internally and supports reverse Tab and Space', async () => {
      await page.setViewportSize({ width: 390, height: 200 });
      await visit('/components', 'get_me');
      const trigger = page.getByRole('button', { name: /^Actions/ });
      await trigger.click();
      const menu = page.getByRole('menu');
      await expect(menu.getByRole('menuitem', { name: 'Edit', exact: true })).toBeFocused();
      await page.keyboard.press('End');
      const last = menu.getByRole('menuitem', { name: 'Delete', exact: true });
      await expect(last).toBeFocused();
      assert.ok(await menu.evaluate(el => el.scrollTop > 0));
      assert.ok(await last.evaluate(el => { const b = el.getBoundingClientRect(); return el.contains(document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2)); }));
      await page.keyboard.press('Shift+Tab');
      await expect(menu).toBeHidden();
      assert.equal(await trigger.evaluate(el => el === document.activeElement), false);
      await trigger.focus();
      await page.keyboard.press('Space');
      const first = menu.getByRole('menuitem', { name: 'Edit', exact: true });
      await expect(first).toBeFocused();
      assert.ok(await first.evaluate(el => { const b = el.getBoundingClientRect(); return el.contains(document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2)); }), 'Reopening makes the focused first item visible');
      await page.keyboard.press('Space');
      await expect(menu).toBeHidden();
      await expect(trigger).toBeFocused();
      await page.keyboard.press('ArrowUp');
      await expect(last).toBeFocused();
      assert.ok(await last.evaluate(el => { const b = el.getBoundingClientRect(); return el.contains(document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2)); }), 'ArrowUp makes the focused last item visible');
      await page.keyboard.press('Escape');
    });
    await check('filter and calendar menus keep their callbacks and survive route changes', async () => {
      await page.setViewportSize({ width: 390, height: 600 });
      await visit('/projects', 'list_projects');
      const filter = page.getByRole('button', { name: /^Active projects/ });
      await filter.click();
      let geometry = await visibleItems(page.getByRole('menu'));
      assert.ok(geometry.inside && geometry.items.every(item => item.hit));
      await page.getByRole('menuitem', { name: /^Archived projects/ }).click();
      await expect(page.getByRole('button', { name: /^Archived projects/ })).toBeVisible();
      await expect(page.getByRole('menu')).toBeHidden();
      await page.getByRole('button', { name: /^Archived projects/ }).click();
      await expect(page.getByRole('menuitem', { name: /^Archived projects/ })).toHaveAttribute('aria-current', 'true');
      await page.getByRole('menuitem', { name: /^Active projects/ }).click();
      await expect(filter).toBeVisible();
      await visit('/timesheet/calendar/2027-10-04', 'list_time_entries');
      for (const name of ['Day view', '5-day view', 'Week view']) {
        await page.locator('button[aria-controls="calendar-span-menu"]').click();
        geometry = await visibleItems(page.getByRole('menu'));
        assert.ok(geometry.inside && geometry.items.every(item => item.hit));
        await page.getByRole('menuitem', { name, exact: true }).click();
        await expect(page.locator('button[aria-controls="calendar-span-menu"]')).toContainText(name);
        await expect(page.getByRole('menu')).toBeHidden();
      }
      await page.locator('button[aria-controls="calendar-span-menu"]').click();
      await page.getByRole('button', { name: 'Week', exact: true }).click();
      await expect(page).toHaveURL(/\/week\//);
      await expect(page.locator('#calendar-span-menu')).toHaveCount(0);
      await page.getByRole('button', { name: 'Calendar', exact: true }).click();
      await page.locator('button[aria-controls="calendar-span-menu"]').click();
      await expect(page.getByRole('menu')).toBeVisible();
      await page.keyboard.press('Escape');
      await expect(page.getByRole('menu')).toBeHidden();
      assert.equal(await page.locator('script[src]').evaluateAll(scripts => scripts.filter(s => /\/menu[^/]*\.js$/.test(s.src)).length), 1);
    });
    assert.deepEqual(errors, []);
    assert.deepEqual(failures, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
