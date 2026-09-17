// Capture before rebuilding, then compare against the same isolated demo database.
// node shared-style-audit.cjs record /tmp/shared-styles.json
// node shared-style-audit.cjs compare /tmp/shared-styles.json
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const fs = require('node:fs');
const assert = require('node:assert/strict');
const [mode, baseline] = process.argv.slice(2);
const base = process.env.HORAE_TEST_URL;
const week = process.env.HORAE_TEST_WEEK;
assert.ok(base && week && baseline && ['record', 'compare'].includes(mode),
  'Set HORAE_TEST_URL and HORAE_TEST_WEEK, and pass record|compare plus a baseline file');
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port !== '8080',
  'Use a separate loopback test instance, not the live app');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  try {
    const page = await browser.newPage();
    // Both runs use fallback fonts, independent of Google Fonts availability.
    await page.route(/https:\/\/fonts\.(googleapis|gstatic)\.com\//, route => route.abort());
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    const result = {};
    for (const [path, resource] of [
      ['/clients', 'list_clients'], ['/invoices', 'list_invoices'],
      ['/reports', 'report_time'], ['/admin/users', 'list_users'],
      ['/settings', 'get_me'], ['/admin/importers', 'get_me'],
      ['/components', 'get_me'], [`/timesheet/week/${week}`, 'list_time_entries'],
    ]) {
      const ready = page.waitForResponse(r => r.url().includes(`/api/${resource}`) && r.status() === 200);
      await page.goto(`${base}${path}`);
      await (await ready).finished();
      await expect(page.locator('.page-header, .ts-header').first()).toBeVisible();
      if (path === '/reports') await expect(page.getByRole('link', { name: 'Export XLSX', exact: true })).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      for (const width of [320, 768, 1440]) {
        await page.setViewportSize({ width, height: 900 });
        result[`${path}:${width}`] = await page.locator(
          '.page-header, .page-title, .page-actions, .page-header .btn, .menu-anchor > .btn, .chip, .ts-header, .ts-toolbar, .ts-toolbar .btn, .app-sidebar, .nav-item, main'
        ).evaluateAll(nodes => nodes.map(node => {
          const style = getComputedStyle(node);
          return Object.fromEntries(['display', 'width', 'height', 'padding', 'gap', 'fontFamily',
            'fontSize', 'fontWeight', 'color', 'backgroundColor', 'borderRadius', 'letterSpacing']
            .map(key => [key, style[key]]));
        }));
      }
    }
    if (mode === 'record') fs.writeFileSync(baseline, JSON.stringify(result, null, 2), { flag: 'wx' });
    else assert.deepEqual(result, JSON.parse(fs.readFileSync(baseline)),
      'Shared layout/typography must remain unchanged outside Projects');
    console.log(`PASS: ${mode}: 8 other screens at 3 widths`);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
