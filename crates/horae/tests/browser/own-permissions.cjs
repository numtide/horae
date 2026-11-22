// Only the browser runner's disposable PostgreSQL fixture may rewrite permission states.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const { readFileSync, mkdirSync } = require('node:fs');
const { join } = require('node:path');
const assert = require('node:assert/strict');
const { setAsideSeededStates } = require('./seeded-permissions.cjs');
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
  let actor, fixtureStarted = false, restoreSeededStates;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const main = page.locator('.adm-main');
  const table = main.getByRole('table', { name: 'Your access by area', exact: true });
  // Opening the account menu re-reads the session, as returning to the page does.
  const reread = async () => {
    await page.locator('.sidebar-footer').click();
    await page.locator('.sidebar-footer').click();
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    // Earlier suites may leave another administrator selected by dev login.
    const identity = await page.context().request.post(`${base}${identityEndpoints[0]}`, { data: {} });
    assert.equal(identity.status(), 200, await identity.text());
    actor = await identity.json();
    for (const id of [actor.id, actor.org_id]) assert.match(id, /^[0-9a-f-]{36}$/);
    assert.equal(sql(`SELECT active FROM users WHERE id='${actor.id}' AND org_id='${actor.org_id}'`), 't');
    restoreSeededStates = setAsideSeededStates(actor.org_id);
    assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${actor.org_id}'`), '0');

    // Old /settings links open Profile, and the side menu reaches every subpage.
    await page.goto(`${base}/settings`);
    await expect(page.getByRole('heading', { name: 'Profile', level: 1 })).toBeVisible();
    for (const [name, path] of [['Preferences', 'preferences'], ['API tokens', 'api-tokens'], ['Your access', 'access'], ['Profile', 'profile']]) {
      await page.locator('.adm-nav').getByRole('link', { name, exact: true }).click();
      await page.waitForURL(`${base}/settings/${path}`);
      await expect(page.getByRole('heading', { name, level: 1 })).toBeVisible();
      await expect(page.locator('.adm-nav').getByRole('link', { name, exact: true })).toHaveAttribute('aria-current', 'page');
    }
    await expect(page.getByRole('heading', { name: 'Plugins', exact: true })).toHaveCount(0);

    // Without a stored state there is nothing to explain; an administrator has to repair it.
    await page.goto(`${base}/settings/access`);
    await expect(main.getByRole('alert')).toContainText('Ask an administrator of this workspace to check your access');

    sql(`INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
      VALUES ('${stateId}', '${actor.org_id}', '${actor.id}', 1, ${floor} || ARRAY['invoice_read_managed'], false, 'individual')`);
    fixtureStarted = true;
    await reread();
    await expect(table).toBeVisible();
    const invoices = table.getByRole('row').filter({ has: page.getByRole('rowheader', { name: 'Invoices', exact: true }) });
    await expect(invoices.getByRole('cell')).toHaveText(['Managed', 'No', '—Does not apply']);
    // The dash is hidden from screen readers, which read the words instead.
    await expect(invoices.getByRole('cell').nth(2).locator('[aria-hidden="true"]')).toHaveText('—');
    const time = table.getByRole('row').filter({ has: page.getByRole('rowheader', { name: 'Time', exact: true }) });
    await expect(time.getByRole('cell')).toHaveText(['Own', 'Own', 'No']);
    await expect(main).toContainText('To change your access, ask an administrator of this workspace.');
    await expect(main.getByText('Individual configuration', { exact: true })).toBeVisible();
    await expect(main.getByRole('link', { name: 'See changes to your access', exact: true })).toHaveCount(0);
    for (const absent of ['Refresh permissions', 'Only an Administrator']) await expect(main).not.toContainText(absent);
    assert.equal(await main.locator('input, select, button').count(), 0);
    assert.ok(!(await main.innerText()).includes(actor.id));

    for (const theme of ['dark', 'light']) {
      for (const [width, height, zoom] of [[320, 900, 1], [390, 844, 1], [768, 900, 1], [1440, 900, 1], [1440, 360, 1], [768, 900, 2]]) {
        await page.setViewportSize({ width, height });
        await page.evaluate(({ theme, zoom }) => {
          document.documentElement.dataset.theme = theme;
          document.documentElement.style.zoom = String(zoom);
        }, { theme, zoom });
        await table.scrollIntoViewIfNeeded();
        await expect(table).toBeVisible();
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, `${theme}/${width}/${zoom}: page overflow`);
        if (process.env.HORAE_BROWSER_ARTIFACTS && zoom === 1 && height > 400 && ((width === 1440 && theme === 'dark') || (width === 390 && theme === 'light'))) {
          mkdirSync(process.env.HORAE_BROWSER_ARTIFACTS, { recursive: true });
          await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, `own-permissions-${width}-${theme}.png`), fullPage: true, animations: 'disabled' });
        }
      }
    }
    await page.evaluate(() => { document.documentElement.style.zoom = ''; });
    await page.setViewportSize({ width: 1440, height: 900 });

    // An administrator sees one sentence instead of the table, and the history.
    sql(`UPDATE person_permission_states SET grants=${floor}, is_administrator=true WHERE id='${stateId}'`);
    await reread();
    await expect(main).toContainText('You can see and change everything here');
    await expect(table).toHaveCount(0);
    await expect(main.getByText('Administrator', { exact: true })).toBeVisible();
    // The link opens the log filtered to the person reading it.
    await expect(main.getByRole('link', { name: 'See changes to your access', exact: true })).toHaveAttribute('href', `/admin/audit?person=${actor.id}`);

    sql(`UPDATE person_permission_states SET catalog_version=2 WHERE id='${stateId}'`);
    await reread();
    await expect(main.getByRole('alert')).toContainText('Ask an administrator of this workspace to check your access');
    await expect(main).not.toContainText('person_permission_states');
    sql(`UPDATE person_permission_states SET catalog_version=1, is_administrator=false WHERE id='${stateId}'`);
    await reread();
    await expect(table).toBeVisible();
    sql(`UPDATE users SET active=false WHERE id='${actor.id}'`);
    // The re-read meets a 401, so the whole page asks to sign in again.
    await page.locator('.sidebar-footer').click();
    await expect(page.getByRole('heading', { name: 'Session ended', exact: true })).toBeVisible();
    await expect(page.getByRole('alert')).toContainText('Sign in again');
    await expect(page.getByRole('button', { name: 'Sign in again', exact: true })).toBeVisible();
    await expect(table).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log(`PASS: Your access in Chromium ${browser.version()}: My settings subpages, missing and stored states, member table, administrator sentence, error recovery, deactivation, both themes and responsive/CSS-zoom layouts`);
  } finally {
    await browser.close();
    if (fixtureStarted) sql(`BEGIN;
      UPDATE users SET active=true WHERE id='${actor.id}';
      DELETE FROM person_permission_states WHERE id='${stateId}' AND org_id='${actor.org_id}'; COMMIT;`);
    restoreSeededStates?.();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
