// Run with run-design-checks.sh modals. Creates and deletes one uniquely named
// entry in its disposable DB; intercepted failures never reach the server.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const failures = [];
  page.on('pageerror', error => errors.push(error.message));
  const longName = 'W'.repeat(200);
  // Keep real eligible IDs/authority while exercising the supported label limit.
  await page.route('**/api/load_timesheet_tracking*', async route => {
    const response = await route.fetch();
    if (response.status() !== 200) return route.fulfill({ response });
    const options = await response.json();
    await route.fulfill({ response, json: options.map(option => ({
      ...option, project_name: longName, task_name: longName,
    })) });
  });
  async function visit(path, resource) {
    const ready = page.waitForResponse(r => r.url().includes(`/api/${resource}`) && r.status() === 200);
    await page.goto(`${base}${path}`);
    await (await ready).finished();
  }
  async function check(name, run) {
    try { await run(); console.log(`PASS: ${name}`); }
    catch (error) { failures.push(name); console.error(`FAIL: ${name}: ${error.stack}`); }
  }
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    for (const scenario of [
      { path: '/projects', resource: 'list_projects', trigger: 'Export', title: 'Export projects' },
      { path: '/timesheet/week/2027-10-04', resource: 'load_timesheet_page', trigger: 'Add entry', title: /New time entry/ },
      { path: '/timesheet/week/2027-10-04', resource: 'load_timesheet_page', trigger: '＋ Add row', title: 'Add a row' },
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
          if (scenario.resource === 'load_timesheet_page') {
            await expect(modal.getByRole('button', { name: 'Project', exact: true })).toContainText(longName);
            await expect(modal.getByRole('button', { name: 'Task', exact: true })).toContainText(longName);
          }
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
      await visit('/timesheet/week/2027-10-04', 'load_timesheet_page');
      const trigger = page.getByRole('button', { name: 'Add entry', exact: true });
      await trigger.click();
      const modal = page.getByRole('dialog', { name: /New time entry/ });
      const project = modal.getByRole('button', { name: 'Project', exact: true });
      await expect(project).toBeEnabled();
      async function selectFirst(label) {
        await modal.getByRole('button', { name: label, exact: true }).click();
        await page.getByRole('dialog', { name: `Choose ${label.toLowerCase()}`, exact: true }).getByRole('option').first().click();
      }
      await selectFirst('Project');
      await selectFirst('Task');
      await modal.getByRole('textbox', { name: 'Duration', exact: true }).fill('1:00');
      const notes = `Modal browser regression ${Date.now()}`;
      await modal.getByPlaceholder('Notes (optional)').fill(notes);
      let release;
      const gate = new Promise(resolve => { release = resolve; });
      const route = '**/api/apply_timesheet_command*';
      let attempts = 0;
      await page.route(route, async r => {
        assert.equal(r.request().postDataJSON().command.operation, 'create');
        attempts += 1;
        await gate;
        await r.abort('failed');
      });
      try {
        await modal.getByRole('button', { name: 'Save entry', exact: true }).click();
        await expect.poll(() => attempts).toBe(1);
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
      await expect.poll(() => attempts).toBe(2);
      await expect(modal.getByRole('alert')).toContainText('Could not save');
      await page.unroute(route);
      // A failed transport also refreshes authority; test Cancel once its opener
      // is available, separately from the refresh fallback on successful writes.
      await expect(trigger).toBeEnabled();
      await page.keyboard.press('Escape');
      await expect(modal).toBeHidden();
      await expect(trigger).toBeFocused();
      await trigger.click();
      await expect(project).toBeEnabled();
      await selectFirst('Project');
      await selectFirst('Task');
      await modal.getByRole('textbox', { name: 'Duration', exact: true }).fill('1:00');
      await modal.getByPlaceholder('Notes (optional)').fill(notes);
      const refreshed = page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
      await modal.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect(modal).toBeHidden();
      // Refresh invalidates tracking options, so the opener is temporarily disabled.
      await expect(page.locator('#app-main')).toBeFocused();
      await (await refreshed).finished();
      await visit('/timesheet/day/2027-10-04', 'load_timesheet_page');
      const row = page.locator('.ts-day-entry').filter({ hasText: notes });
      const edit = row.getByRole('button', { name: 'Edit', exact: true });
      await edit.click();
      const editing = page.getByRole('dialog', { name: /Edit time entry/ });
      await expect(editing.getByRole('button', { name: 'Project', exact: true })).toBeEnabled();
      // Catalog arrival can change the first available control; focus must stay
      // inside the native dialog rather than depend on network timing.
      await expect.poll(() => editing.evaluate(el => el.contains(document.activeElement))).toBe(true);
      await page.keyboard.press('Escape');
      await expect(edit).toBeFocused();
      await edit.click();
      await page.route(route, r => {
        assert.equal(r.request().postDataJSON().command.operation, 'update');
        return r.abort('failed');
      });
      await editing.getByRole('textbox', { name: 'Duration', exact: true }).fill('2:00');
      await editing.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect(editing.getByRole('alert')).toContainText('Could not save');
      await expect(editing.getByRole('textbox', { name: 'Duration', exact: true })).toHaveValue('2:00');
      await page.unroute(route);
      await editing.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect(editing).toBeHidden();
      await expect(page.locator('#app-main')).toBeFocused();
      await expect(row.locator('.ts-day-entry-dur')).toHaveText('2:00');
      await edit.click();
      await page.setViewportSize({ width: 320, height: 320 });
      const deleted = page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
      await editing.getByRole('button', { name: 'Delete', exact: true }).click();
      await expect(editing).toBeHidden();
      const afterDelete = await (await deleted).json();
      assert.equal(afterDelete.next_after, null);
      assert.equal(afterDelete.entries.filter(entry => entry.notes === notes).length, 0);
      await expect(trigger).toBeEnabled();
      await expect(row).toHaveCount(0);
      await expect(page.locator('#app-main')).toBeFocused();
      await trigger.click();
      await expect(modal).toBeVisible();
      await page.keyboard.press('Escape');
      await expect(trigger).toBeFocused();
    });
    assert.deepEqual(errors, []);
    assert.deepEqual(failures, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
