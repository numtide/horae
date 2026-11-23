// Sections, radius and width (DESIGN.md: Section titles, Border Radius, Form
// width): every panel at 16px, section titles at card-title, forms at most
// 720px and tables at the full content width; Workspace settings is one form
// with one Primary, and Plugins is its own page in the Workspace subnav.
// With HORAE_BROWSER_ARTIFACTS it first saves a 1280px dark capture of each
// screen; HORAE_CAPTURE_ONLY=1 stops there, for capturing an older build.
// This suite only reads the database created by run-design-checks.sh.
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { expect } = playwright;
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const { join } = require('node:path');
const evidence = process.env.HORAE_BROWSER_ARTIFACTS;
if (evidence) mkdirSync(evidence, { recursive: true });
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
// The seeded client: earlier suites leave clients in other organizations.
const client = sql("SELECT id FROM clients WHERE name = 'Acme Corp'");
const project = sql(`SELECT id FROM projects WHERE client_id = '${client}' ORDER BY name LIMIT 1`);

const screens = [
  ['workspace-settings', '/admin/workspace'],
  ['workspace-plugins', '/admin/plugins'],
  ['admin-api-tokens', '/admin/api-tokens'],
  ['my-profile', '/settings/profile'],
  ['my-preferences', '/settings/preferences'],
  ['my-api-tokens', '/settings/api-tokens'],
  ['invoice-settings', '/invoices/settings'],
  ['new-invoice', '/invoices/new'],
  ['client-detail', `/clients/${client}`],
  ['project-detail', `/projects/${project}`],
  ['new-project', '/projects/new'],
  ['harvest', '/admin/importers/harvest'],
  ['harvest-oauth', '/admin/importers/harvest/oauth'],
];

(async () => {
  const browser = await playwright.chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 }, colorScheme: 'dark' });
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const open = async path => {
    await page.goto(`${base}${path}`);
    await expect(page.locator('.page-header').first()).toBeVisible();
    await page.waitForLoadState('networkidle');
    await expect(page.locator('main [role="status"].loading')).toHaveCount(0);
  };
  const width = async locator => (await locator.boundingBox()).width;
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    if (evidence) {
      for (const [name, path] of screens) {
        // An older build may lack a screen (Plugins had no page of its own).
        try { await open(path); } catch (error) {
          if (!process.env.HORAE_CAPTURE_ONLY) throw error;
          console.log(`skipped ${name}: ${error.message.split('\n')[0]}`);
          continue;
        }
        await page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
        await page.screenshot({ path: join(evidence, `${name}-1280-dark.png`), fullPage: true, animations: 'disabled' });
      }
    }
    if (process.env.HORAE_CAPTURE_ONLY) {
      console.log('PASS: captures saved');
      return;
    }

    // Every panel and section title follows the one rule on every screen.
    for (const [name, path] of screens) {
      await open(path);
      for (const panel of await page.locator('main :is(.card, .table-container, .empty-state:not(.inset), .skeleton, .metric-card, .audit-log)').all()) {
        if (!await panel.isVisible()) continue;
        await expect(panel, `${name}: panel radius`).toHaveCSS('border-radius', '16px');
      }
      for (const title of await page.locator('main .card-title').all()) {
        await expect(title, `${name}: section title`).toHaveCSS('font-size', '18px');
        await expect(title, `${name}: section title`).toHaveCSS('font-weight', '500');
      }
      for (const list of await page.locator('main .card ul').all()) {
        await expect(list, `${name}: a list of data has no bullets`).toHaveCSS('list-style-type', 'none');
      }
    }

    // Wide enough that the admin panel exceeds 720px, so the form limit shows.
    await page.setViewportSize({ width: 1680, height: 1000 });

    // Workspace settings: one form card of at most 720px, one Primary, and
    // Plugins as its own subpage beside Settings.
    await open('/admin/workspace');
    assert.ok(await width(page.locator('.adm-main')) > 760, 'the admin panel is wider than the old 760px limit');
    const main = page.locator('.adm-main');
    await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible();
    await expect(main.locator('.card')).toHaveCount(1);
    assert.ok(await width(main.locator('.card')) <= 720, 'the settings form is at most 720px');
    await expect(main.locator('.btn-primary')).toHaveCount(1);
    await expect(main.getByRole('heading', { name: 'Plugins' })).toHaveCount(0);
    const subnav = page.locator('.adm-nav');
    await expect(subnav.getByRole('link', { name: 'Settings', exact: true })).toHaveAttribute('aria-current', 'page');
    await subnav.getByRole('link', { name: 'Plugins', exact: true }).click();
    await page.waitForURL(`${base}/admin/plugins`);
    await expect(page.getByRole('heading', { name: 'Plugins', level: 1 })).toBeVisible();
    await expect(subnav.getByRole('link', { name: 'Plugins', exact: true })).toHaveAttribute('aria-current', 'page');
    // No plugin is installed in the test server: the empty state fills the panel.
    const empty = main.locator('.empty-state');
    await expect(empty).toContainText('No plugins installed');
    assert.ok(Math.abs(await width(empty) - await width(main)) <= 1, 'the empty state is full width');

    // The admin shell sets no width: the API tokens table uses all of it.
    await open('/admin/api-tokens');
    const tokens = main.locator(':is(.table-container, .empty-state)').first();
    assert.ok(Math.abs(await width(tokens) - await width(main)) <= 1, 'the tokens list is full width');
    await expect(main, 'the admin panel has no width limit').toHaveCSS('max-width', 'none');

    // My settings: the forms are at most 720px; the token form is one row whose
    // button lines up with the fields, and the list below it is full width.
    for (const path of ['/settings/profile', '/settings/preferences']) {
      await open(path);
      assert.ok(await width(main.locator('.card').first()) <= 720, `${path}: form at most 720px`);
    }
    await open('/settings/api-tokens');
    const form = page.getByRole('region', { name: 'New token', exact: true });
    assert.ok(await width(form) <= 720, 'the token form is at most 720px');
    const name = (await form.getByLabel('Name', { exact: true }).boundingBox());
    const expiry = (await form.locator('#api-token-expiry').boundingBox());
    const create = (await form.getByRole('button', { name: 'Create token' }).boundingBox());
    for (const box of [expiry, create]) {
      assert.ok(Math.abs(box.y + box.height - (name.y + name.height)) <= 1, 'the row lines up on its controls');
    }
    const list = page.getByRole('region', { name: 'Your tokens', exact: true }).locator(':is(.table-container, .empty-state)').first();
    assert.ok(Math.abs(await width(list) - await width(main)) <= 1, 'the token list is full width');

    // Workspace and invoice settings draw their forms at the same limit.
    await open('/invoices/settings');
    for (const card of await page.locator('main form.card').all()) {
      assert.ok(await width(card) <= 720, 'invoice settings form at most 720px');
    }

    assert.deepEqual(errors, []);
    console.log('PASS: sections, panel radius and form width on 13 screens; Workspace settings and Plugins');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
