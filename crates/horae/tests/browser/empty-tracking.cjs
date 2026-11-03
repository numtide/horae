// Run against an isolated, seeded dev-login instance with Playwright installed.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/empty-tracking.cjs
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
    // An organization with nothing assigned yet: both catalogs answer empty.
    await page.route(/\/api\/(list_tracking_projects|load_timesheet_tracking)/, route => route.fulfill({ json: [] }));
    const ready = page.waitForResponse(r => r.url().includes('/api/load_timesheet_tracking'));
    await page.goto(`${base}/`);
    await (await ready).finished();

    const add = page.getByRole('button', { name: 'Add entry', exact: true });
    await expect(add).toBeDisabled();
    await expect(add).toHaveAttribute('title', 'No projects to track yet');
    const hint = page.locator(`#${await add.getAttribute('aria-describedby')}`);
    await expect(hint).toBeVisible();
    await expect(hint).toContainText('No projects to track yet.');
    // The dev admin may create projects, so the hint leads there.
    await expect(hint.getByRole('link', { name: 'Create a project', exact: true })).toHaveAttribute('href', '/projects/new');
    console.log('PASS: the disabled add button explains an empty catalog and links to project creation');

    const sidebar = page.locator('.app-sidebar');
    await sidebar.getByRole('button', { name: 'Start timer', exact: true }).click();
    const popover = sidebar.locator('.sidebar-timer-pop');
    await expect(popover).toBeVisible();
    await expect(popover).toContainText('No projects to track yet.');
    await expect(popover.getByRole('link', { name: 'Create a project', exact: true })).toBeVisible();
    // Nothing to pick, so there is no picker and no way to start.
    await expect(popover.locator('select, input, textarea')).toHaveCount(0);
    await expect(popover.getByRole('button', { name: 'Start timer', exact: true })).toHaveCount(0);
    await popover.getByRole('link', { name: 'Create a project', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/new`);
    await expect(popover).toHaveCount(0);
    console.log('PASS: the timer popover shows the empty state instead of an empty picker');

    await page.unroute(/\/api\/(list_tracking_projects|load_timesheet_tracking)/);
    const loaded = page.waitForResponse(r => r.url().includes('/api/load_timesheet_tracking') && r.status() === 200);
    await page.goto(`${base}/`);
    await (await loaded).finished();
    await expect(add).toBeEnabled();
    await expect(add).not.toHaveAttribute('title', /./);
    await expect(page.locator('.no-tracking-hint')).toHaveCount(0);
    console.log('PASS: with projects to track the add button carries no empty-state hint');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error({ url: page.url(), errors });
    throw error;
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
