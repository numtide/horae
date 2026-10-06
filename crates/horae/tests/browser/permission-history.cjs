// Exercise history without depending on the separate permission editor UI.
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
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id, org_id FROM users WHERE email='admin@example.com' AND active AND org_role='admin') u"));
const project = sql(`SELECT id FROM projects WHERE org_id='${actor.org_id}' AND active ORDER BY id LIMIT 1`);
for (const id of [actor.id, actor.org_id, project]) assert.match(id, /^[0-9a-f-]{36}$/);
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${actor.org_id}'`), '0');
for (const table of ['person_permission_states', 'permission_change_receipts', 'project_management_assignments']) {
  assert.equal(sql(`SELECT count(*) FROM ${table} WHERE org_id='${actor.org_id}'`), '0');
}
const initialRevision = sql(`SELECT access_revision FROM organizations WHERE id='${actor.org_id}'`);
assert.match(initialRevision, /^\d+$/);
const stateId = '01960000-0000-7000-8000-000000000901';
// Dioxus 0.7.9 appends a build-specific decimal hash to implicit endpoints.
// Read the actual server's registered string; never hard-code a build's hash.
const endpoints = [...new Set(readFileSync(process.env.HORAE_TEST_SERVER).toString('latin1').match(/\/api\/save_project_managers\d+/g))];
assert.equal(endpoints.length, 1, 'expected one compiled project-manager endpoint');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15_000);
  page.setDefaultNavigationTimeout(30_000);
  const errors = [];
  let releaseRead;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const history = page.getByRole('table', { name: 'Permission change history', exact: true });
  const receiptIds = () => history.locator('details p').filter({ hasText: /^Receipt: / }).allTextContents();
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    sql(`BEGIN;
      INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
      VALUES ('${stateId}', '${actor.org_id}', '${actor.id}', 1,
        ARRAY['time_read_own','time_write_own','expense_read_own','expense_write_own','project_read_all','project_write_all'], true, 'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${actor.org_id}'; COMMIT;`);
    await page.goto(`${base}/admin/audit`);
    await expect(page.getByRole('status')).toContainText('No permission events recorded yet');
    await expect(page.locator('#audit-older')).toBeDisabled();

    let revision = Number(initialRevision);
    assert.ok(Number.isSafeInteger(revision));
    // Real session-bound commands produce 27 changes and one no-op receipt.
    // No receipt JSON is manufactured or inserted by the fixture.
    for (let index = 0; index < 28; index++) {
      const changed = index < 27;
      const response = await page.context().request.post(`${base}${endpoints[0]}`, {
        data: {
          expected_requester: { org_id: actor.org_id, user_id: actor.id },
          command: {
            kind: 'replace_project_managers',
            request_id: `01960000-0000-7000-8000-${String(1000 + index).padStart(12, '0')}`,
            expected_access_revision: revision,
            project_id: project,
            manager_ids: index % 2 === 0 || !changed ? [actor.id] : [],
          },
        },
      });
      assert.equal(response.status(), 200, await response.text());
      const outcome = await response.json();
      assert.equal(outcome.changed, changed);
      assert.equal(outcome.access_revision, revision + Number(changed));
      revision = outcome.access_revision;
    }
    const ids = sql(`SELECT id FROM permission_change_receipts WHERE org_id='${actor.org_id}' ORDER BY created_at DESC, id DESC`).split('\n');
    assert.equal(ids.length, 28);
    await page.locator('#audit-refresh').click();
    await expect(history.locator('tbody tr')).toHaveCount(25);
    assert.deepEqual(await receiptIds(), ids.slice(0, 25).map(id => `Receipt: ${id}`));
    await expect(history.locator('summary').first()).toHaveText('No project manager change');
    const summary = history.locator('summary').nth(1);
    await summary.focus();
    await page.keyboard.press('Enter');
    await expect(history.locator('details').nth(1)).toHaveAttribute('open', '');
    await expect(history.getByText(`Project: ${project}`, { exact: true }).nth(1)).toBeVisible();
    await expect(history.locator('details').nth(1)).toContainText(`Added manager: ${actor.id}`);

    for (const [width, height, theme] of [[1440, 900, 'dark'], [390, 844, 'light']]) {
      await page.setViewportSize({ width, height });
      await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, 'history must not widen the viewport');
      for (const id of ['audit-refresh', 'audit-newest', 'audit-older']) {
        const box = await page.locator(`#${id}`).boundingBox();
        assert.ok(box && box.x >= 0 && box.x + box.width <= width + 1, `${id} must remain within the viewport`);
      }
      if (process.env.HORAE_BROWSER_ARTIFACTS) {
        mkdirSync(process.env.HORAE_BROWSER_ARTIFACTS, { recursive: true });
        await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, `permission-history-${width}-${theme}.png`), animations: 'disabled' });
      }
    }
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.locator('#audit-older').focus();
    await page.keyboard.press('Enter');
    await expect(history.locator('tbody tr')).toHaveCount(3);
    assert.deepEqual(await receiptIds(), ids.slice(25).map(id => `Receipt: ${id}`));
    await expect(page.locator('#audit-older')).toBeDisabled();
    await page.locator('#audit-newest').click();
    await expect(history.locator('tbody tr')).toHaveCount(25);

    let intercepted = false;
    const heldRead = new Promise(resolve => { releaseRead = resolve; });
    const holdRead = async route => { intercepted = true; await heldRead; await route.continue(); };
    await page.route('**/api/list_permission_audit*', holdRead);
    try {
      await page.locator('#audit-refresh').click();
      await expect.poll(() => intercepted).toBe(true);
      await expect(page.getByRole('status')).toContainText('Loading permission history');
      await expect(history).not.toBeVisible();
      for (const id of ['audit-refresh', 'audit-newest', 'audit-older']) await expect(page.locator(`#${id}`)).toBeDisabled();
    } finally {
      releaseRead();
      await page.unroute('**/api/list_permission_audit*', holdRead);
    }
    await expect(history.locator('tbody tr')).toHaveCount(25);
    sql(`UPDATE person_permission_states SET is_administrator=false WHERE id='${stateId}'`);
    await page.locator('#audit-refresh').click();
    await expect(page.getByRole('alert')).toContainText('Administrator access is required');
    await expect(history).not.toBeVisible();
    sql(`UPDATE person_permission_states SET is_administrator=true WHERE id='${stateId}'`);
    await page.locator('#audit-refresh').click();
    await expect(history.locator('tbody tr')).toHaveCount(25);
    assert.deepEqual(await receiptIds(), ids.slice(0, 25).map(id => `Receipt: ${id}`));

    sql(`UPDATE users SET org_role='member' WHERE id='${actor.id}'`);
    await page.goto(`${base}/settings`);
    await page.getByRole('link', { name: 'View permission audit log', exact: true }).click();
    await page.waitForURL(`${base}/admin/audit`);
    await expect(history.locator('tbody tr')).toHaveCount(25);
    sql(`UPDATE users SET active=false WHERE id='${actor.id}'`);
    await page.locator('#audit-refresh').click();
    await expect(page.getByRole('alert')).toContainText('Sign in again');
    await expect(history).not.toBeVisible();
    assert.deepEqual(errors, []);
    console.log(`PASS: permission history in Chromium ${browser.version()}: real writer receipts, empty/no-op states, exact paging, keyboard details, responsive themes, stale suppression, revocation, canonical admin navigation and deactivation`);
  } finally {
    releaseRead?.();
    await browser.close();
    sql(`BEGIN;
      UPDATE users SET active=true, org_role='admin' WHERE id='${actor.id}';
      DELETE FROM project_management_assignments WHERE org_id='${actor.org_id}' AND manager_id='${actor.id}' AND project_id='${project}';
      DELETE FROM permission_change_receipts WHERE org_id='${actor.org_id}' AND actor_user_id='${actor.id}' AND request_id::text LIKE '01960000-0000-7000-8000-%';
      DELETE FROM person_permission_states WHERE id='${stateId}' AND org_id='${actor.org_id}';
      UPDATE organizations SET permission_policy_version=0, access_revision=${initialRevision} WHERE id='${actor.org_id}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
