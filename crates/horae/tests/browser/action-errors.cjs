// Run against a dev-login-enabled, seeded test instance, never production:
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/action-errors.cjs
// Requires Playwright; PLAYWRIGHT_MODULE and CHROMIUM_PATH can select local installs.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated, seeded test instance');

(async () => {
  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_PATH || undefined,
    headless: true,
  });
  const context = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const page = await context.newPage();
  const failures = [];
  const pageErrors = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  const cases = [
    {
      name: 'client activation', path: '/clients', resource: 'list_clients',
      endpoint: 'set_client_active', form: 'Add Client',
      formEndpoint: 'create_client', submit: 'Create Client',
      action: async () => page.getByRole('button', { name: 'Deactivate', exact: true }).first().click(),
    },
    {
      name: 'project archive', path: '/projects', resource: 'get_project_overview',
      endpoint: 'set_project_active', form: /^Actions/,
      formEndpoint: 'save_project_editor', submit: 'Save changes', separatePage: true,
      openForm: async () => {
        await page.locator('.proj-row').getByRole('button', { name: /^Actions/ }).first().click();
        await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
      },
      action: async () => {
        await page.locator('.proj-row').getByRole('button', { name: /^Actions/ }).first().click();
        await page.getByRole('menuitem', { name: 'Archive', exact: true }).click();
      },
    },
  ];
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    for (const scenario of cases) {
      const pattern = `**/api/${scenario.endpoint}*`;
      const formPattern = `**/api/${scenario.formEndpoint}*`;
      await page.route(pattern, route => route.abort('failed'));
      await page.route(formPattern, route => route.abort('failed'));
      try {
        const ready = page.waitForResponse(r => r.url().includes(`/api/${scenario.resource}`) && r.status() === 200);
        await page.goto(`${base}${scenario.path}`);
        await (await ready).finished();
        await expect(page.getByRole('button', { name: scenario.form, exact: true }).first()).toBeVisible();
        const rejected = page.waitForEvent('requestfailed', r => r.url().includes(`/api/${scenario.endpoint}`));
        await scenario.action();
        await rejected;
        await expect(page.locator('.alert-danger')).toBeVisible();
        await expect(page.getByRole('alert')).toBeVisible();
        if (scenario.separatePage) {
          // The editor now has its own route. A list popover must not hide a
          // failed archive; the editor must independently preserve failed input.
          await page.locator('.proj-row').getByRole('button', { name: /^Actions/ }).first().click();
          await page.keyboard.press('Escape');
          await expect(page.getByRole('alert')).toBeVisible();
          await scenario.openForm();
          await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}\/edit$/);
          const editor = page.locator('.np-page');
          const originalName = await editor.locator('#np-name').inputValue();
          const formRejected = page.waitForEvent('requestfailed', r => r.url().includes(`/api/${scenario.formEndpoint}`));
          await editor.getByRole('button', { name: scenario.submit, exact: true }).click();
          await formRejected;
          await expect(editor.getByRole('alert')).toBeVisible();
          await expect(editor.locator('#np-name')).toHaveValue(originalName);
          await expect(editor.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
          await page.unroute(formPattern);
          await editor.getByRole('button', { name: 'Retry request', exact: true }).click();
          await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}$/);
          await expect(page.getByRole('alert')).toHaveCount(0);
          console.log('PASS: archive failure survives menu dismissal; shared edit failure preserves input and safely retries');
          continue;
        }
        // Opening and cancelling an unrelated form must not clear an action error.
        if (scenario.openForm) await scenario.openForm();
        else await page.getByRole('button', { name: scenario.form, exact: true }).click();
        await expect(page.getByRole('alert')).toBeVisible();
        const formRejected = page.waitForEvent('requestfailed', r => r.url().includes(`/api/${scenario.formEndpoint}`));
        await page.getByRole('button', { name: scenario.submit, exact: true }).click();
        await formRejected;
        await expect(page.locator('.card .alert-danger')).toBeVisible();
        await expect(page.locator('.alert-danger')).toHaveCount(2);
        await page.getByRole('button', { name: 'Cancel', exact: true }).click();
        await expect(page.getByRole('alert')).toBeVisible();
        await expect(page.locator('.alert-danger')).toHaveCount(1);
        console.log(`PASS: ${scenario.name} failure remains visible with form closed, open and cancelled`);
        if (scenario.endpoint === 'set_client_active') {
          // Use a new test client for the real successful retry, leaving seed data alone.
          await page.unroute(formPattern);
          const name = `Action error browser ${Date.now()}`;
          await page.getByRole('button', { name: 'Add Client', exact: true }).click();
          await page.getByLabel('Name', { exact: true }).fill(name);
          const created = page.waitForResponse(r => r.url().includes('/api/create_client'));
          await page.getByRole('button', { name: 'Create Client', exact: true }).click();
          assert.equal((await created).status(), 200);
          const row = page.getByRole('row').filter({ has: page.getByRole('cell', { name, exact: true }) });
          await expect(row).toBeVisible();
          await expect(page.getByRole('alert')).toBeVisible();
          await page.unroute(pattern);
          const changed = page.waitForResponse(r => r.url().includes('/api/set_client_active'));
          await row.getByRole('button', { name: 'Deactivate', exact: true }).click();
          assert.equal((await changed).status(), 200);
          await expect(row.getByText('Inactive', { exact: true })).toBeVisible();
          await expect(page.getByRole('alert')).toHaveCount(0);
          const restored = page.waitForResponse(r => r.url().includes('/api/set_client_active'));
          await row.getByRole('button', { name: 'Activate', exact: true }).click();
          assert.equal((await restored).status(), 200);
          await expect(row.getByText('Active', { exact: true })).toBeVisible();
          console.log('PASS: successful form save preserves action error; real status retry clears it and refreshes the row');
        }
      } catch (error) {
        failures.push(`${scenario.name}: ${error.message}`);
      } finally {
        await page.unroute(pattern);
        await page.unroute(formPattern);
      }
    }
    // Team changes share the project editor's atomic save and recovery flow.
    await page.goto(`${base}/projects/new`);
    const newProject = page.locator('.np-page');
    await newProject.getByRole('button', { name: 'Client', exact: true }).click();
    await page.getByRole('dialog', { name: 'Choose Client', exact: true }).getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await newProject.getByLabel('Project name', { exact: true }).fill(`Team recovery ${Date.now()}`);
    await newProject.getByRole('radio', { name: /^Non-Billable/ }).check();
    await newProject.locator('#np-add-person').click();
    await page.getByRole('dialog', { name: 'Choose teammate', exact: true }).locator('[role="option"]:not(:disabled)').first().click();
    await newProject.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}$/);
    const projectPath = new URL(page.url()).pathname;
    await page.getByRole('link', { name: 'Edit project', exact: true }).click();
    const editor = page.locator('.np-page');
    const remove = editor.getByRole('button', { name: /^Remove .+ from project$/ }).first();
    await expect(remove).toBeVisible();
    const label = await remove.getAttribute('aria-label');
    const teammate = label.match(/^Remove (.+) from project$/)[1];
    await remove.click();
    await page.route('**/api/save_project_editor*', route => route.abort('failed'));
    const failedSave = page.waitForEvent('requestfailed', r => r.url().includes('/api/save_project_editor'));
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await failedSave;
    await expect(editor.getByRole('alert')).toBeVisible();
    await expect(editor.getByRole('button', { name: label, exact: true })).toHaveCount(0);
    await expect(editor.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    await page.unroute('**/api/save_project_editor*');
    await editor.getByRole('button', { name: 'Retry request', exact: true }).click();
    await expect(page).toHaveURL(`${base}${projectPath}`);
    await page.getByRole('link', { name: 'Edit project', exact: true }).click();
    await expect(editor.getByRole('heading', { name: 'Edit project', exact: true })).toBeVisible();
    await expect(editor.getByRole('button', { name: label, exact: true })).toHaveCount(0);
    await editor.locator('#np-add-person').click();
    await page.getByRole('dialog', { name: 'Choose teammate', exact: true }).getByRole('option', { name: teammate, exact: true }).click();
    await expect(editor.getByRole('button', { name: label, exact: true })).toBeVisible();
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}${projectPath}`);
    await expect(page.getByRole('alert')).toHaveCount(0);
    console.log('PASS: failed team removal preserves the draft, safely retries and can be restored through the shared editor');
    assert.deepEqual(pageErrors, []);
    assert.deepEqual(failures, []);
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
