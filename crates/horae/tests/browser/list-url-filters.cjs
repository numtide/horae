// Lists that keep their filters in the URL: Back and Forward return to the
// search each entry had, and a route change alone never rewrites it. Uses only
// the database created by run-design-checks.sh; its one draft invoice (so the
// Invoices list shows its filters) is removed at the end.
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { expect } = playwright;
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const invoice = '019f5300-0000-7000-8000-000000000001';

// (path, search label, status trigger, status item, the query the status writes)
const lists = [
  ['/clients', 'Search clients by name', '#client-scope-menu-trigger', /^All clients/, 'status=all'],
  ['/projects', 'Search by project or client', '#project-scope-menu-trigger', /^Archived projects/, 'status=archived'],
  ['/invoices', 'Search by number or client', '#invoices-status-filter-trigger', /^Draft$/, 'status=draft'],
];

(async () => {
  const browser = await playwright.chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  sql(`INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency)
    SELECT '${invoice}', org_id, id, 'URL-FILTERS', 'draft', '2026-10-01', '2026-10-31', currency
    FROM clients WHERE active ORDER BY id LIMIT 1`);
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    for (const [path, label, trigger, item, status] of lists) {
      await page.goto(`${base}${path}`);
      const search = page.getByRole('searchbox', { name: label, exact: true });
      await expect(page.locator(trigger)).toBeEnabled();
      await search.fill('first');
      await page.waitForURL(url => url.search === '?q=first');
      await page.locator(trigger).click();
      await page.getByRole('menuitem', { name: item }).click();
      await page.waitForURL(url => url.search === `?${status}&q=first`);
      await search.fill('second');
      await page.waitForURL(url => url.search === `?${status}&q=second`);

      await page.goBack();
      await page.waitForURL(url => url.search === '?q=first');
      await expect(search).toHaveValue('first');
      // Past the search pause, nothing has rewritten the entry Back reached.
      await page.waitForTimeout(800);
      assert.equal(new URL(page.url()).search, '?q=first', `${path}: Back keeps its search`);

      await page.goForward();
      await page.waitForURL(url => url.search === `?${status}&q=second`);
      await expect(search).toHaveValue('second');
      await page.waitForTimeout(800);
      assert.equal(new URL(page.url()).search, `?${status}&q=second`, `${path}: Forward keeps its search`);

      // Choosing the status already shown adds no history entry.
      const entries = await page.evaluate(() => history.length);
      await page.locator(trigger).click();
      await page.getByRole('menuitem', { name: item }).click();
      await page.waitForTimeout(300);
      assert.equal(await page.evaluate(() => history.length), entries, `${path}: a repeated pick adds no entry`);
      console.log(`PASS: ${path} Back and Forward return to each entry's search`);
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    sql(`DELETE FROM invoices WHERE id='${invoice}'`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
