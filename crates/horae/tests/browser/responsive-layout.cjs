// Run against an isolated, seeded dev-login instance. No business data is changed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/responsive-layout.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  const failures = [];
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
  async function headersFit() {
    await expect(page.locator('.page-header, .ts-header').first()).toBeVisible();
    const problems = await page.locator('.page-header, .ts-header, .ts-toolbar').evaluateAll(headers => {
      const problems = [];
      if (document.documentElement.scrollWidth > window.innerWidth + 1)
        problems.push(`Page width ${document.documentElement.scrollWidth} exceeds viewport ${window.innerWidth}`);
      for (const header of headers) {
        const bounds = header.getBoundingClientRect();
        const controls = [...header.querySelectorAll('.page-title, button, a, input')]
          .filter(el => el.getClientRects().length);
        for (const el of controls) {
          const box = el.getBoundingClientRect();
          if (box.left < bounds.left - 1 || box.right > bounds.right + 1)
            problems.push(`${el.textContent || el.getAttribute('aria-label')}: outside ${header.className}`);
          if (el.tagName !== 'INPUT' && el.scrollWidth > el.clientWidth + 1)
            problems.push(`${el.textContent}: clipped control text`);
        }
        for (let i = 0; i < controls.length; i++) {
          for (let j = i + 1; j < controls.length; j++) {
            const a = controls[i].getBoundingClientRect(), b = controls[j].getBoundingClientRect();
            if (Math.min(a.right, b.right) > Math.max(a.left, b.left) + 1 &&
                Math.min(a.bottom, b.bottom) > Math.max(a.top, b.top) + 1)
              problems.push(`${controls[i].textContent} overlaps ${controls[j].textContent}`);
          }
        }
      }
      return problems;
    });
    assert.deepEqual(problems, []);
  }
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    for (const [path, resource] of [
      ['/clients', 'list_clients'], ['/projects', 'list_projects'],
      ['/invoices', 'list_invoices'], ['/reports', 'report_time'],
      ['/admin/users', 'list_users'], ['/approvals', 'list_approvals'],
      ['/settings', 'get_me'], ['/admin/importers', 'get_me'],
      ['/timesheet/week/2027-10-04', 'list_time_entries'],
      ['/timesheet/day/2027-10-04', 'list_time_entries'],
      ['/timesheet/calendar/2027-10-04', 'list_time_entries'],
    ]) {
      await visit(path, resource);
      if (path === '/reports') await expect(page.getByRole('link', { name: 'Export XLSX', exact: true })).toBeVisible();
      for (const width of [320, 390, 768, 1280, 1440]) {
        await page.setViewportSize({ width, height: 1000 });
        await check(`${path} header at ${width}px`, headersFit);
        if (process.env.HORAE_TEST_SCREENSHOT_DIR && width === 320 && path === '/timesheet/week/2027-10-04')
          await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/responsive-timesheet.png` });
      }
    }
    await check('project names and badges stay within aligned columns; scrolling to last-row actions opens editing', async () => {
      await page.setViewportSize({ width: 1280, height: 1000 });
      await visit('/projects', 'list_projects');
      await expect(page.locator('.proj-row').first()).toBeVisible();
      if (process.env.HORAE_TEST_SCREENSHOT_DIR)
        await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/responsive-projects.png` });
      // Stress only rendered labels, not stored project data or server behavior.
      await page.locator('.proj-namelink').first().evaluate(el => {
        el.textContent = 'A long project name with multiple words and an_unbroken_reference_'.repeat(3);
      });
      for (const width of [320, 390, 768, 1280, 1440]) {
        await page.setViewportSize({ width, height: 1000 });
        const geometry = await page.locator('.proj-scroll').evaluate(scroll => {
          const head = [...scroll.querySelector('.proj-head').children].map(el => el.getBoundingClientRect());
          return [...scroll.querySelectorAll('.proj-row')].flatMap(row => {
            const cells = [...row.children].map(el => el.getBoundingClientRect());
            const problems = [];
            cells.forEach((cell, i) => {
              if (Math.abs(cell.x - head[i].x) > 1 || Math.abs(cell.width - head[i].width) > 1)
                problems.push(`column ${i} differs from header`);
            });
            for (const el of row.firstElementChild.children) {
              const box = el.getBoundingClientRect();
              if (box.left < cells[0].left - 1 || box.right > cells[0].right + 1 || el.scrollWidth > el.clientWidth + 1)
                problems.push(`${el.textContent}: outside name column`);
            }
            return problems;
          });
        });
        assert.deepEqual(geometry, [], `Project columns at ${width}px`);
      }
      await page.setViewportSize({ width: 320, height: 1000 });
      const last = page.locator('.proj-row').last();
      await last.getByRole('button', { name: 'Actions' }).click();
      await last.getByRole('menu').getByRole('menuitem', { name: 'Edit', exact: true }).click();
      await expect(page.getByRole('heading', { name: 'Edit Project', exact: true })).toBeVisible();
      await page.locator('.page-header').getByRole('button', { name: 'Cancel', exact: true }).click();
    });
    await check('small-screen timesheet pager, date picker and view controls remain usable', async () => {
      await page.setViewportSize({ width: 320, height: 1000 });
      await visit('/timesheet/week/2027-10-04', 'list_time_entries');
      await page.getByRole('button', { name: 'Next week', exact: true }).click();
      await expect(page).toHaveURL(/2027-10-11/);
      await page.getByRole('button', { name: 'Previous week', exact: true }).click();
      await expect(page).toHaveURL(/2027-10-04/);
      await page.locator('.ts-pager-label').click();
      const picker = page.locator('.dp-pop');
      await expect(picker).toBeVisible();
      const box = await picker.boundingBox();
      assert.ok(box.x >= 0 && box.x + box.width <= 321, 'Date picker fits viewport');
      await page.mouse.click(4, 4);
      await page.getByRole('button', { name: 'Calendar', exact: true }).click();
      await expect(page).toHaveURL(/\/calendar\//);
      await page.getByRole('button', { name: 'Week view', exact: false }).click();
      await page.getByRole('menu').getByRole('menuitem', { name: 'Day view', exact: true }).click();
      await expect(page).toHaveURL(/span=day/);
      await headersFit();
    });
    await check('admin subnavigation and its content remain reachable on mobile', async () => {
      await page.setViewportSize({ width: 320, height: 1000 });
      await visit('/admin/users', 'list_users');
      await expect(page.getByRole('button', { name: 'Invite User', exact: true })).toBeVisible();
      await page.locator('.adm-nav').getByRole('link', { name: 'Importers', exact: true }).click();
      await expect(page).toHaveURL(`${base}/admin/importers`);
      await expect(page.locator('.adm-main .page-title')).toHaveText('Importers');
      const panel = await page.locator('.adm-main').boundingBox();
      assert.ok(panel.width >= 288 && panel.x >= 0 && panel.x + panel.width <= 320);
    });
    await check('day strip scrolls to Sunday and long entry labels do not cover actions', async () => {
      await page.setViewportSize({ width: 320, height: 1000 });
      await visit('/timesheet/day/2027-10-04', 'list_time_entries');
      const first = page.locator('.ts-day-entry').first();
      await expect(first).toBeVisible();
      await first.locator('.ts-day-entry-project').evaluate(el => { el.textContent = 'unbroken_project_reference_'.repeat(10); });
      await headersFit();
      if (process.env.HORAE_TEST_SCREENSHOT_DIR)
        await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/responsive-day.png` });
      await first.getByRole('button', { name: 'Edit', exact: true }).click();
      await expect(page.getByRole('dialog', { name: /Edit time entry/ })).toBeVisible();
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
      await page.locator('.ts-daystrip').getByRole('button', { name: /^Sun/ }).click();
      await expect(page).toHaveURL(/2027-10-10/);
      await expect(page.locator('.ts-dayitem.active .ts-dayitem-name')).toHaveText('Sun');
      await headersFit();
    });
    assert.deepEqual(errors, []);
    assert.deepEqual(failures, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
