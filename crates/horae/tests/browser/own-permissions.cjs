// Only the browser runner's disposable PostgreSQL fixture may enable policy 1.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const { readFileSync, mkdirSync } = require('node:fs');
const { join } = require('node:path');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
// Implicit Dioxus endpoints carry a build-specific hash.
const serverStrings = readFileSync(process.env.HORAE_TEST_SERVER).toString('latin1');
const identityEndpoints = [...new Set(serverStrings.match(/\/api\/get_me\d+/g))];
assert.equal(identityEndpoints.length, 1, 'expected one compiled session-identity endpoint');
const stateId = '01960000-0000-7000-8000-000000000801';
const floor = "ARRAY['time_read_own','time_write_own','expense_read_own','expense_write_own']";

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  let releaseRead;
  let actor, fixtureStarted = false;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const section = page.getByRole('region', { name: 'Your permissions', exact: true });
  const refresh = section.getByRole('button', { name: 'Refresh permissions', exact: true });
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    // Earlier suites may leave another administrator selected by dev login.
    const identity = await page.context().request.post(`${base}${identityEndpoints[0]}`, { data: {} });
    assert.equal(identity.status(), 200, await identity.text());
    actor = await identity.json();
    for (const id of [actor.id, actor.org_id]) assert.match(id, /^[0-9a-f-]{36}$/);
    assert.equal(actor.org_role, 'admin');
    assert.equal(sql(`SELECT active FROM users WHERE id='${actor.id}' AND org_id='${actor.org_id}'`), 't');
    assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${actor.org_id}'`), '0');
    assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${actor.org_id}'`), '0');
    await page.goto(`${base}/settings`);
    await expect(section).toContainText('Detailed permissions are not enabled');
    await expect(page.getByRole('heading', { name: 'General', exact: true })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Plugins', exact: true })).toBeVisible();

    sql(`BEGIN;
      INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
      VALUES ('${stateId}', '${actor.org_id}', '${actor.id}', 1, ${floor} || ARRAY['invoice_read_managed'], false, 'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${actor.org_id}'; COMMIT;`);
    fixtureStarted = true;
    await refresh.focus();
    await page.keyboard.press('Shift+Tab');
    await page.keyboard.press('Tab');
    await expect(refresh).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(section).toContainText('View invoices for managed projects');
    await expect(section.getByRole('listitem')).toHaveCount(5);
    await expect(section).not.toContainText('Administrator access');
    assert.equal(await section.locator('input, select, a').count(), 0);
    assert.ok(!(await section.innerText()).includes(actor.id));

    for (const theme of ['dark', 'light']) {
      for (const [width, height, zoom] of [[320, 900, 1], [390, 844, 1], [768, 900, 1], [1440, 900, 1], [1440, 360, 1], [768, 900, 2]]) {
        await page.setViewportSize({ width, height });
        await page.evaluate(({ theme, zoom }) => {
          document.documentElement.dataset.theme = theme;
          document.documentElement.style.zoom = String(zoom);
        }, { theme, zoom });
        await refresh.scrollIntoViewIfNeeded();
        await expect(refresh).toBeVisible();
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, `${theme}/${width}/${zoom}: page overflow`);
        assert.equal(await section.evaluate(el => el.scrollWidth <= el.clientWidth + 1), true, `${theme}/${width}/${zoom}: section overflow`);
        if (process.env.HORAE_BROWSER_ARTIFACTS && zoom === 1 && height > 400 && ((width === 1440 && theme === 'dark') || (width === 390 && theme === 'light'))) {
          mkdirSync(process.env.HORAE_BROWSER_ARTIFACTS, { recursive: true });
          await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, `own-permissions-${width}-${theme}.png`), fullPage: true, animations: 'disabled' });
        }
      }
    }
    await page.evaluate(() => { document.documentElement.style.zoom = ''; });
    await page.setViewportSize({ width: 1440, height: 900 });

    const heldRead = new Promise(resolve => { releaseRead = resolve; });
    const holdRead = async route => { await heldRead; await route.continue(); };
    await page.route('**/api/get_my_permissions*', holdRead);
    try {
      await refresh.click();
      await expect(refresh).toBeDisabled();
      await expect(section.getByRole('status')).toContainText('Loading your permissions');
      await expect(section).not.toContainText('View invoices for managed projects');
      sql(`UPDATE person_permission_states SET grants=${floor}, is_administrator=true WHERE id='${stateId}'`);
    } finally {
      releaseRead();
    }
    await expect(section).toContainText('Administrator access');
    await expect(section.getByRole('listitem')).toHaveCount(4);
    await page.unroute('**/api/get_my_permissions*', holdRead);

    sql(`UPDATE person_permission_states SET catalog_version=2 WHERE id='${stateId}'`);
    await refresh.click();
    await expect(section.getByRole('alert')).toContainText('Could not load your permissions');
    await expect(section).not.toContainText('View your own time');
    await expect(section).not.toContainText('person_permission_states');
    sql(`UPDATE person_permission_states SET catalog_version=1 WHERE id='${stateId}'`);
    await refresh.click();
    await expect(section).toContainText('View your own time');
    sql(`UPDATE users SET active=false WHERE id='${actor.id}'`);
    await refresh.click();
    await expect(section.getByRole('alert')).toContainText('Sign in again');
    await expect(section).not.toContainText('View your own time');
    assert.deepEqual(errors, []);
    console.log(`PASS: own permissions in Chromium ${browser.version()}: real reads, legacy/canonical states, exact grants, independent admin identity, keyboard refresh, pending suppression, error recovery, deactivation, both themes and responsive/CSS-zoom layouts`);
  } finally {
    releaseRead?.();
    await browser.close();
    if (fixtureStarted) sql(`BEGIN;
      UPDATE users SET active=true WHERE id='${actor.id}';
      UPDATE organizations SET permission_policy_version=0 WHERE id='${actor.org_id}';
      DELETE FROM person_permission_states WHERE id='${stateId}' AND org_id='${actor.org_id}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
