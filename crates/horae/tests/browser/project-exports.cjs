// All records, role changes and downloads belong to the disposable runner.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs/promises');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.ok(database.pathname === '/horae' && database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const project = sql("SELECT id FROM projects WHERE code='ACME-01'");
const admin = sql("SELECT id FROM users WHERE email='admin@example.com'");
const originalName = sql(`SELECT name FROM projects WHERE id='${project}'`);
const day = sql(`SELECT min(spent_date) FROM time_entries WHERE project_id='${project}'`);
const foreign = '01980000-0000-7000-8000-000000000003';
sql(`INSERT INTO organizations (id,name) VALUES ('01980000-0000-7000-8000-000000000001','Export foreign org');
INSERT INTO clients (id,org_id,name,currency) VALUES ('01980000-0000-7000-8000-000000000002','01980000-0000-7000-8000-000000000001','Foreign client','EUR');
INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ('${foreign}','01980000-0000-7000-8000-000000000001','01980000-0000-7000-8000-000000000002','Foreign project','EUR');`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const endpoint = `/api/projects/${project}/export/pdf`;
  try {
    assert.equal((await context.request.get(`${base}${endpoint}`)).status(), 401);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/projects/${project}`);
    const trigger = page.locator('#project-export-trigger');
    await expect(trigger).toBeEnabled();
    for (const custom of [false, true]) {
      if (custom) {
        await page.locator('#project-report-period-trigger').click();
        await page.getByRole('menuitem', { name: 'Custom…', exact: true }).click();
        await page.locator('#project-report-from').fill(day);
        await page.locator('#project-report-to').fill(day);
        await page.getByRole('button', { name: 'Apply period', exact: true }).click();
        await expect(trigger).toBeEnabled();
      }
      for (const label of ['CSV', 'Excel', 'PDF summary']) {
        await trigger.focus();
        await page.keyboard.press('ArrowDown');
        await page.getByRole('menuitem', { name: label, exact: true }).click();
        const dialog = page.getByRole('dialog', { name: `Export project — ${label}`, exact: true });
        await expect(dialog).toBeVisible();
        await expect(dialog).toContainText(custom ? day : 'All time');
        if (label === 'Excel') await expect(dialog).toContainText('10,000');
        if (label === 'PDF summary') await expect(dialog).toContainText('1,000');
        const link = dialog.getByRole('link', { name: `Download ${label}`, exact: true });
        const url = new URL(await link.getAttribute('href'), base);
        assert.ok(url.pathname.includes(project) || url.searchParams.get('project_id') === project);
        assert.equal(url.searchParams.get('from'), custom ? day : null);
        assert.equal(url.searchParams.get('to'), custom ? day : null);
        const downloadEvent = page.waitForEvent('download');
        await link.click();
        const download = await downloadEvent;
        assert.equal(await download.failure(), null);
        const bytes = await fs.readFile(await download.path());
        if (label === 'PDF summary') assert.equal(bytes.subarray(0, 5).toString(), '%PDF-');
        else if (label === 'Excel') assert.equal(bytes.subarray(0, 2).toString(), 'PK');
        else {
          const rows = Number(sql(`SELECT count(*) FROM time_entries WHERE project_id='${project}'${custom ? ` AND spent_date='${day}'` : ''}`));
          assert.equal(bytes.toString().trim().split('\n').length - 1, rows);
        }
        assert.equal(new URL(page.url()).pathname, `/projects/${project}`);
        await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
        await expect(trigger).toBeFocused();
      }
    }
    // dx injects offscreen development toasts into this debug artifact; they are
    // not application UI and must not affect production-layout measurements.
    await page.addStyleTag({ content: '#__dx-toast { display: none !important; }' });
    for (const [width, height, fontSize] of [[320, 900, 16], [390, 500, 16], [768, 900, 16], [1440, 900, 16], [320, 500, 32], [1440, 900, 32]]) {
      await page.setViewportSize({ width, height });
      await page.evaluate(size => { document.documentElement.style.fontSize = `${size}px`; }, fontSize);
      await trigger.click();
      await page.getByRole('menuitem', { name: 'PDF summary', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Export project — PDF summary', exact: true });
      const overflow = await page.evaluate(() => ({
        width: innerWidth, scroll: document.documentElement.scrollWidth,
        elements: [...document.querySelectorAll('body *')].filter(element => {
          const bounds = element.getBoundingClientRect();
          if (!bounds.width || bounds.right <= innerWidth || getComputedStyle(element).position === 'fixed') return false;
          for (let parent = element.parentElement; parent; parent = parent.parentElement) {
            if (['auto', 'scroll', 'hidden', 'clip'].includes(getComputedStyle(parent).overflowX) && parent.getBoundingClientRect().right <= innerWidth) return false;
          }
          return true;
        }).slice(0, 12).map(element => ({ tag: element.tagName, class: element.className, text: element.textContent.slice(0, 80) })),
      }));
      const bounds = await dialog.locator('.modal').boundingBox();
      assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width);
      assert.ok(await dialog.locator('.modal').evaluate(element => element.scrollWidth <= element.clientWidth), 'Modal contents must reflow without horizontal scrolling');
      assert.ok(overflow.scroll <= width, JSON.stringify({ width, height, fontSize, overflow }));
      const downloadLink = dialog.getByRole('link', { name: 'Download PDF summary', exact: true });
      await downloadLink.scrollIntoViewIfNeeded();
      const downloadBounds = await downloadLink.boundingBox();
      assert.ok(downloadBounds.y >= 0 && downloadBounds.y + downloadBounds.height <= height, 'Download must remain reachable in a short viewport');
      await page.keyboard.press('Escape');
      await expect(trigger).toBeFocused();
    }
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    for (const query of ['from=&to=', 'from=2026-09-01', 'from=2026-09-30&to=2026-09-01', 'task_id=ignored']) {
      assert.equal((await context.request.get(`${base}${endpoint}?${query}`)).status(), 400);
    }
    assert.equal((await context.request.get(`${base}/api/projects/${foreign}/export/pdf`)).status(), 404);
    sql(`UPDATE projects SET name=repeat('x',32768) WHERE id='${project}'`);
    const oversized = await context.request.get(`${base}${endpoint}`);
    assert.equal(oversized.status(), 413);
    assert.ok((await oversized.text()).includes('No partial PDF'));
    sql(`UPDATE projects SET name='${originalName.replace(/'/g, "''")}' WHERE id='${project}'`);
    sql(`UPDATE users SET org_role='member' WHERE id='${admin}'`);
    assert.equal((await context.request.get(`${base}${endpoint}`)).status(), 403);
    await page.reload();
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(trigger).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log('Project exports: CSV/Excel/PDF native downloads, all-time/custom scope, keyboard/focus, responsive bounds, invalid filters, foreign scope, limits and revoked role passed');
  } finally {
    sql(`UPDATE users SET org_role='admin' WHERE id='${admin}'; UPDATE projects SET name='${originalName.replace(/'/g, "''")}' WHERE id='${project}'`);
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
