// Run against an isolated, seeded dev-login instance. Creates and deletes one
// uniquely named entry; intercepted failures never reach the server.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/modals.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
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
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    for (const scenario of [
      { path: '/projects', resource: 'list_projects', trigger: 'Export', title: 'Export projects' },
      { path: '/timesheet/week/2027-10-04', resource: 'list_time_entries', trigger: 'Add entry', title: /New time entry/ },
      { path: '/timesheet/week/2027-10-04', resource: 'list_time_entries', trigger: '＋ Add row', title: 'Add a row' },
    ]) {
      await check(`${scenario.trigger}: semantics, keyboard, focus return, backdrop and short viewport`, async () => {
        await page.setViewportSize({ width: 1440, height: 900 });
        await visit(scenario.path, scenario.resource);
        const trigger = page.getByRole('button', { name: scenario.trigger, exact: true });
        const modal = page.getByRole('dialog', { name: scenario.title });
        for (let cycle = 0; cycle < 2; cycle++) {
          await trigger.click();
          await expect(modal).toBeVisible();
          assert.ok(await modal.evaluate(el => el.contains(document.activeElement)), 'Opening moves focus inside');
          await trigger.evaluate(el => el.focus());
          assert.ok(await modal.evaluate(el => el.contains(document.activeElement)), 'Background is inert');
          for (let i = 0; i < 30; i++) {
            await page.keyboard.press(i < 15 ? 'Tab' : 'Shift+Tab');
            assert.ok(await modal.evaluate(el => el.contains(document.activeElement) || document.activeElement === document.body), 'Tab cannot reach background controls');
          }
          await page.keyboard.press('Escape');
          await expect(modal).toBeHidden();
          await expect(trigger).toBeFocused();
        }
        await trigger.click();
        await modal.getByRole('button', { name: 'Cancel', exact: true }).click();
        await expect(modal).toBeHidden();
        await expect(trigger).toBeFocused();
        await trigger.click();
        const panelBounds = await modal.locator('.modal').boundingBox();
        await page.mouse.move(panelBounds.x + panelBounds.width / 2, panelBounds.y + 20);
        await page.mouse.down();
        await page.mouse.move(4, 4);
        await page.mouse.up();
        await expect(modal).toBeVisible();
        await page.mouse.click(4, 4);
        await expect(modal).toBeHidden();
        await expect(trigger).toBeFocused();
        for (const width of [390, 320]) {
          await page.setViewportSize({ width, height: 320 });
          await trigger.click();
          await expect(modal).toBeVisible();
          const panel = modal.locator('.modal');
          const bounds = await panel.boundingBox();
          assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width + 1, 'Panel fits viewport width');
          assert.ok(bounds.y >= 0 && bounds.y + bounds.height <= 320, 'Panel fits viewport height');
          assert.ok(await panel.evaluate(el => el.scrollWidth <= el.clientWidth), 'Panel does not scroll horizontally');
          await modal.getByRole('button', { name: 'Cancel', exact: true }).click();
          await expect(modal).toBeHidden();
          await expect(trigger).toBeFocused();
        }
        await page.getByRole('button', { name: 'Open navigation', exact: true }).click();
        await trigger.click();
        await page.keyboard.press('Escape');
        await expect(modal).toBeHidden();
        await expect(page.getByRole('button', { name: 'Close navigation', exact: true })).toBeVisible();
        await expect(trigger).toBeFocused();
      });
    }
    await check('pending save blocks dismissal; failure preserves input and allows retry', async () => {
      await page.setViewportSize({ width: 1440, height: 900 });
      await visit('/timesheet/week/2027-10-04', 'list_time_entries');
      const trigger = page.getByRole('button', { name: 'Add entry', exact: true });
      await trigger.click();
      const modal = page.getByRole('dialog', { name: /New time entry/ });
      const project = modal.getByRole('combobox', { name: 'Project', exact: true });
      await expect(project).toBeEnabled();
      const projectId = await project.locator('option').evaluateAll(options => options.find(o => o.value)?.value);
      await project.selectOption(projectId);
      const task = modal.getByRole('combobox', { name: 'Task', exact: true });
      const taskId = await task.locator('option').evaluateAll(options => options.find(o => o.value)?.value);
      await task.selectOption(taskId);
      await modal.getByRole('textbox', { name: 'Duration', exact: true }).fill('1:00');
      const notes = `Modal browser regression ${Date.now()}`;
      await modal.getByPlaceholder('Notes (optional)').fill(notes);
      let release;
      const gate = new Promise(resolve => { release = resolve; });
      const route = url => url.pathname.includes('/api/create_time_entry');
      await page.route(route, async r => { await gate; await r.abort('failed'); });
      try {
        const sent = page.waitForRequest(r => r.url().includes('/api/create_time_entry'));
        await modal.getByRole('button', { name: 'Save entry', exact: true }).click();
        await sent;
        await expect(modal.getByRole('button', { name: 'Saving…', exact: true })).toBeDisabled();
        await expect(modal.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
        await page.keyboard.press('Escape');
        await page.mouse.click(4, 4);
        await expect(modal).toBeVisible();
      } finally { release(); }
      await expect(modal.getByRole('alert')).toContainText('Could not save');
      await expect(modal.getByPlaceholder('Notes (optional)')).toHaveValue(notes);
      await expect(modal.getByRole('button', { name: 'Save entry', exact: true })).toBeEnabled();
      await modal.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect(modal.getByRole('alert')).toContainText('Could not save');
      await page.unroute(route);
      await page.keyboard.press('Escape');
      await expect(modal).toBeHidden();
      await expect(trigger).toBeFocused();
      await trigger.click();
      await expect(project).toBeEnabled();
      await project.selectOption(projectId);
      await task.selectOption(taskId);
      await modal.getByRole('textbox', { name: 'Duration', exact: true }).fill('1:00');
      await modal.getByPlaceholder('Notes (optional)').fill(notes);
      const refreshed = page.waitForResponse(r => r.url().includes('/api/list_time_entries') && r.status() === 200);
      await modal.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect(modal).toBeHidden();
      await expect(trigger).toBeFocused();
      await (await refreshed).finished();
      await visit('/timesheet/day/2027-10-04', 'list_time_entries');
      const row = page.locator('.ts-day-entry').filter({ hasText: notes });
      const edit = row.getByRole('button', { name: 'Edit', exact: true });
      await edit.click();
      const editing = page.getByRole('dialog', { name: /Edit time entry/ });
      await expect(editing.getByRole('combobox', { name: 'Project', exact: true })).toBeDisabled();
      await expect(editing.getByPlaceholder('Notes (optional)')).toBeFocused();
      await page.keyboard.press('Escape');
      await expect(edit).toBeFocused();
      await edit.click();
      const updateRoute = url => url.pathname.includes('/api/update_time_entry');
      await page.route(updateRoute, r => r.abort('failed'));
      await editing.getByRole('textbox', { name: 'Duration', exact: true }).fill('2:00');
      await editing.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect(editing.getByRole('alert')).toContainText('Could not save');
      await expect(editing.getByRole('textbox', { name: 'Duration', exact: true })).toHaveValue('2:00');
      await page.unroute(updateRoute);
      await editing.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect(editing).toBeHidden();
      await expect(row.locator('.ts-day-entry-dur')).toHaveText('2:00');
      await edit.click();
      await page.setViewportSize({ width: 320, height: 320 });
      await editing.getByRole('button', { name: 'Delete', exact: true }).click();
      await expect(editing).toBeHidden();
      await expect(row).toHaveCount(0);
      await trigger.click();
      await expect(modal).toBeVisible();
      await page.keyboard.press('Escape');
      await expect(trigger).toBeFocused();
    });
    assert.deepEqual(errors, []);
    assert.deepEqual(failures, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
