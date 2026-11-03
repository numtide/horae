// Adds a second person, so only in the runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const org = '01950000-0000-7000-8000-000000000001';
const other = '019f2a00-0000-7000-8000-000000000001', otherName = 'Empty tracking teammate';
sql(`INSERT INTO users (id,org_id,email,name) VALUES ('${other}','${org}','empty-tracking@example.test','${otherName}') ON CONFLICT (id) DO NOTHING`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push({ message: error.message, stack: error.stack, url: page.url() }));
  const catalogs = /\/api\/(list_tracking_projects|load_timesheet_tracking)/;
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    // Nothing assigned to the viewer: both catalogs answer empty.
    await page.route(catalogs, route => route.fulfill({ json: [] }));
    const me = page.waitForResponse(r => r.url().includes('/api/get_me') && r.status() === 200);
    const ready = page.waitForResponse(r => r.url().includes('/api/load_timesheet_tracking'));
    await page.goto(`${base}/`);
    const { id } = await (await me).json();
    await (await ready).finished();
    const sheet = page.url();

    const add = page.getByRole('button', { name: 'Add entry', exact: true });
    await expect(add).toBeDisabled();
    await expect(add).toHaveAttribute('title', 'No projects are assigned to you yet.');
    const hint = page.locator(`#${await add.getAttribute('aria-describedby')}`);
    await expect(hint).toHaveText(/^No projects are assigned to you yet\./);
    // The dev admin manages projects, so the hint leads to them.
    await expect(hint.getByRole('link', { name: 'Open Projects', exact: true })).toHaveAttribute('href', '/projects');
    console.log('PASS: the disabled add button explains missing assignments and links to Projects');

    // Opening one's own sheet by id is still one's own.
    const today = new Date().toISOString().slice(0, 10);
    const open = async user => {
      const loaded = page.waitForResponse(r => r.url().includes('/api/load_timesheet_tracking'));
      await page.goto(`${base}/timesheet/week/${today}?span=week&user=${user}`);
      await (await loaded).finished();
    };
    await open(id);
    await expect(add).toHaveAttribute('title', 'No projects are assigned to you yet.');
    await expect(page.locator('#ts-add-empty').getByRole('link', { name: 'Open Projects', exact: true })).toBeVisible();
    console.log('PASS: the own sheet opened by id still speaks to the viewer');

    // Someone else's sheet names them and offers no link.
    await open(other);
    await expect(add).toHaveAttribute('title', `No projects are assigned to ${otherName} yet.`);
    await expect(page.locator('#ts-add-empty')).toHaveText(`No projects are assigned to ${otherName} yet.`);
    await expect(page.locator('#ts-add-empty a')).toHaveCount(0);
    console.log("PASS: another person's empty sheet names them without a link");

    const own = page.waitForResponse(r => r.url().includes('/api/list_tracking_projects'));
    await page.goto(sheet);
    await (await own).finished();
    const sidebar = page.locator('.app-sidebar');
    await sidebar.getByRole('button', { name: 'Start timer', exact: true }).click();
    const popover = sidebar.locator('.sidebar-timer-pop');
    await expect(popover).toContainText('No projects are assigned to you yet.');
    // Nothing to pick, so there is no picker and no way to start.
    await expect(popover.locator('select, input, textarea')).toHaveCount(0);
    await expect(popover.getByRole('button', { name: 'Start timer', exact: true })).toHaveCount(0);
    await popover.getByRole('link', { name: 'Open Projects', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await expect(popover).toHaveCount(0);
    console.log('PASS: the timer popover shows the empty state instead of an empty picker');

    await page.unroute(catalogs);
    const loaded = page.waitForResponse(r => r.url().includes('/api/load_timesheet_tracking') && r.status() === 200);
    await page.goto(sheet);
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
