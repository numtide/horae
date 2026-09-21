// Mutates only the runner's disposable, seeded database on its reserved port.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to the isolated design-check runner');
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093',
  'New-project checks require the disposable runner on port 8093');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const requests = new Map();
  context.on('page', tab => {
    const pending = new Set();
    requests.set(tab, pending);
    tab.on('request', request => {
      if (new URL(request.url()).pathname.startsWith('/api/')) pending.add(request);
    });
    tab.on('requestfinished', request => pending.delete(request));
    tab.on('requestfailed', request => pending.delete(request));
    tab.on('pageerror', error => errors.push(error.stack || error.message));
    tab.on('console', message => {
      if (message.type() === 'error' && !message.text().includes('Failed to load resource') && !message.text().startsWith('WebSocket connection')) console.error(message.text());
    });
  });
  const page = await context.newPage();
  // Destroying a document midway through a response body triggers an upstream
  // Dioxus decoder panic. Await reads when tearing down test pages; deliberately
  // interrupted mutations below still exercise the application's retry paths.
  const readsFinished = tab => expect.poll(() => requests.get(tab).size).toBe(0);
  const screen = page.locator('.np-page');
  const saved = () => expect(screen.getByRole('status')).toContainText('Draft saved at');
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await page.getByRole('link', { name: 'Projects', exact: true }).click();
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/new`);
    await expect(screen.getByRole('status')).toHaveText('No draft saved yet');
    await expect(screen.getByRole('button', { name: 'Save project', exact: true })).toBeDisabled();

    await screen.getByRole('button', { name: '+ New client', exact: true }).click();
    const client = page.getByRole('dialog', { name: 'New client', exact: true });
    await client.getByLabel('Client name', { exact: true }).fill('New project browser client');
    await client.getByLabel('Currency', { exact: true }).selectOption('EUR');
    await client.getByLabel('Default hourly rate', { exact: true }).fill('0');
    await client.getByRole('button', { name: /Create client/ }).click();
    await expect(client).not.toBeVisible();
    await expect(screen.getByLabel('Client', { exact: true })).toContainText('New project browser client');
    await screen.getByLabel('Project name', { exact: true }).fill('Recoverable browser project');
    await screen.getByLabel('Project code', { exact: true }).fill('BROWSER-NEW');
    await screen.getByLabel('Start date', { exact: true }).fill('2026-09-01');
    await screen.getByLabel('Tags', { exact: true }).fill('browser');
    await screen.getByLabel('Tags', { exact: true }).press('Enter');
    await screen.getByRole('radio', { name: /^Project hourly rate/ }).check();
    await screen.locator('#np-project-rate').fill('75.25');
    await screen.getByLabel('Budget', { exact: true }).selectOption({ label: 'Total project hours' });
    await screen.locator('#np-budget-value').fill('120');
    await expect(screen.getByRole('checkbox', { name: /^Email me/ })).toBeDisabled();
    await screen.getByRole('button', { name: 'Add everyone', exact: true }).click();
    await expect(screen.getByLabel('Project name', { exact: true })).toBeEnabled();
    await expect(screen.getByRole('checkbox', { name: /manages this project/ }).first()).toBeVisible();
    await screen.getByRole('checkbox', { name: /manages this project/ }).first().check();
    await screen.getByRole('region', { name: 'Tasks', exact: true }).getByRole('button', { name: 'Development', exact: true }).click();
    await screen.locator('#np-task-search').fill('Browser custom task');
    await screen.getByRole('button', { name: 'Add task', exact: true }).click();
    await screen.getByRole('button', { name: /^Access for Browser custom task:/ }).click();
    const access = page.getByRole('dialog', { name: 'Who can track to this task?' });
    await access.getByRole('radio', { name: 'Only selected people', exact: true }).check();
    await access.getByRole('checkbox').first().check();
    await access.getByRole('button', { name: 'Apply access', exact: true }).click();
    await expect(access).not.toBeVisible();
    await screen.getByLabel('Payment terms', { exact: true }).selectOption({ label: 'Custom days' });
    await screen.getByLabel('Days until payment is due', { exact: true }).fill('21');
    await screen.getByLabel('Tax (%)', { exact: true }).fill('21');
    await screen.getByRole('button', { name: 'Add a second tax', exact: true }).click();
    await screen.getByLabel('Second tax name', { exact: true }).fill('Local tax');
    await screen.getByLabel('Second tax (%)', { exact: true }).fill('1.5');
    await saved();
    await readsFinished(page);
    // Simulate a catalog page that does not contain the saved selections. The
    // selected-ID endpoint still reads the real, isolated database.
    await page.route('**/api/project_creation_options*', async route => {
      const response = await route.fetch();
      const options = await response.json();
      return route.fulfill({ response, json: { ...options, tasks: [], people: [], more_tasks: true, more_people: true } });
    });
    const selectedReady = page.waitForResponse(response => response.url().includes('/api/project_creation_selection') && response.status() === 200);
    await page.reload();
    const resolved = await (await selectedReady).json();
    assert.deepEqual(resolved.tasks.map(task => task.name), ['Development']);
    assert.deepEqual(resolved.people.map(person => person.name), ['Admin User']);
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recoverable browser project');
    await expect(screen.getByLabel('Start date', { exact: true })).toHaveValue('2026-09-01');
    await expect(screen.locator('#np-project-rate')).toHaveValue('75.25');
    await expect(screen.locator('#np-budget-value')).toHaveValue('120');
    await expect(screen.getByLabel('Days until payment is due', { exact: true })).toHaveValue('21');
    await expect(screen.getByLabel('Second tax name', { exact: true })).toHaveValue('Local tax');
    await expect(screen.getByRole('button', { name: /^Access for Browser custom task:/ })).not.toContainText('Everyone');
    await expect(screen.getByRole('button', { name: 'Remove task Development', exact: true })).toBeVisible();
    await expect(screen.getByRole('checkbox', { name: 'Admin User manages this project', exact: true })).toBeChecked();
    await expect(screen).not.toContainText('Unavailable task');
    await expect(screen).not.toContainText('Unavailable teammate');
    await readsFinished(page);
    await page.unroute('**/api/project_creation_options*');
    console.log('PASS: real client, team, task restrictions, billing and invoice defaults survive reload');

    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true,
        `New project fits at ${width}px`);
      // Screenshots restore caret styling by leaving empty style attributes.
      await expect(screen.locator('[style]:not([style=""])')).toHaveCount(0);
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
        await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-${width}.png`, fullPage: true });
      }
    }
    await screen.getByRole('radio', { name: /^Fixed Fee/ }).check();
    await screen.getByRole('radio', { name: 'Milestones', exact: true }).check();
    await screen.getByRole('button', { name: 'Add milestone', exact: true }).click();
    await screen.getByLabel('Milestone name', { exact: true }).fill('Delivery');
    await screen.getByLabel('Due date', { exact: true }).fill('2026-10-01');
    await screen.getByLabel('Amount (EUR)', { exact: true }).fill('1000');
    await screen.getByRole('radio', { name: /^Non-Billable/ }).check();
    await expect(screen.getByRole('heading', { name: 'Invoice defaults', exact: true })).toHaveCount(0);
    await screen.getByRole('radio', { name: /^Time & Materials/ }).check();
    await expect(screen.locator('#np-project-rate')).toHaveValue('75.25');
    await saved();
    await screen.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recoverable browser project');
    console.log('PASS: responsive form, conditional panels and Cancel preserve the draft');

    let release;
    let hold = true;
    await page.route('**/api/save_project_draft*', async route => {
      const response = await route.fetch();
      if (hold) await new Promise(resolve => { release = resolve; });
      return route.fulfill({ response });
    });
    try {
      await screen.getByLabel('Project name', { exact: true }).fill('Older snapshot');
      await expect.poll(() => !!release).toBe(true);
      await screen.getByLabel('Project name', { exact: true }).fill('Edited during save');
      await expect(screen.getByRole('status')).toHaveText('Saving draft…');
      hold = false;
      release();
      await saved();
      await readsFinished(page);
      await page.reload();
      await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Edited during save');
    } finally {
      hold = false;
      if (release) release();
      await page.unroute('**/api/save_project_draft*');
    }
    console.log('PASS: edits made while a save is pending are saved after its acknowledgement');

    // Commit the first request, then lose its response. A retry must acknowledge
    // that same snapshot before saving edits made during the uncertain outcome.
    let lost;
    let retry;
    await page.route('**/api/save_project_draft*', async route => {
      if (!lost) {
        lost = route.request().postData();
        await route.fetch();
        return route.abort();
      }
      retry ??= route.request().postData();
      return route.continue();
    });
    await screen.getByLabel('Project name', { exact: true }).fill('Acknowledgement lost');
    await expect(screen.getByRole('alert')).toBeVisible();
    await expect(screen.getByRole('status')).toHaveText('Changes need attention');
    await screen.getByLabel('Project name', { exact: true }).fill('Recovered latest edit');
    await screen.getByRole('button', { name: 'Retry request', exact: true }).click();
    await saved();
    assert.equal(retry, lost, 'The first retry reuses the exact uncertain request');
    await readsFinished(page);
    await page.reload();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recovered latest edit');
    await page.unroute('**/api/save_project_draft*');
    console.log('PASS: lost acknowledgement retries the same request and preserves newer edits');

    let created;
    await page.route('**/api/finalize_project_draft*', async route => {
      const response = await route.fetch();
      const id = await response.json();
      assert.equal(response.status(), 200, JSON.stringify(id));
      if (!created) {
        created = id;
        return route.abort();
      }
      assert.equal(id, created, 'Retry returns the already-created project');
      return route.fulfill({ response });
    });
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(screen.getByRole('alert')).toBeVisible();
    await expect(screen.getByLabel('Project name', { exact: true })).toBeDisabled();
    await screen.getByRole('button', { name: 'Retry request', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${created}`);
    await expect(page.getByRole('heading', { name: 'Project', exact: true })).toBeVisible();
    await readsFinished(page);
    await page.goto(`${base}/projects/new`);
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('');
    await expect(screen.getByRole('status')).toHaveText('No draft saved yet');
    console.log('PASS: lost creation response is idempotent and clears the completed draft');

    await screen.getByLabel('Project name', { exact: true }).fill('Two tab draft');
    await saved();
    const second = await context.newPage();
    try {
      await second.goto(`${base}/projects/new`);
      await expect(second.getByLabel('Project name', { exact: true })).toHaveValue('Two tab draft');
      await screen.getByLabel('Project name', { exact: true }).fill('First tab acknowledged');
      await saved();
      await second.getByLabel('Project name', { exact: true }).fill('Stale tab edit');
      await expect(second.locator('.np-page').getByRole('alert')).toBeVisible();
      await expect(second.locator('.np-page').getByRole('status')).toHaveText('Changes need attention');
      await second.getByRole('button', { name: 'Reload saved draft (lose local edits)', exact: true }).click();
      await expect(second.getByLabel('Project name', { exact: true })).toHaveValue('First tab acknowledged');
    } finally {
      await readsFinished(second);
      await second.close();
    }
    await screen.getByRole('button', { name: 'Discard draft', exact: true }).click();
    const discard = page.getByRole('dialog', { name: 'Discard this draft?', exact: true });
    await discard.getByRole('button', { name: 'Keep editing', exact: true }).click();
    await expect(discard).not.toBeVisible();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('First tab acknowledged');
    await screen.getByRole('button', { name: 'Discard draft', exact: true }).click();
    await discard.getByRole('button', { name: 'Discard draft', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await expect(page.getByRole('heading', { name: 'Projects', exact: true })).toBeVisible();
    await readsFinished(page);
    await page.goto(`${base}/projects/new`);
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('');
    await expect(screen.getByRole('status')).toHaveText('No draft saved yet');
    console.log('PASS: stale tabs cannot overwrite saved input; explicit discard removes only the draft');
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
