// Exercise history without depending on the separate permission editor UI.
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
const stateId = '01960000-0000-7000-8000-000000000901';
// Removes an organization's rows from `tables` and returns a function that puts them back.
const setAside = (org, tables) => {
  const saved = tables.map(table => [table, sql(`WITH removed AS (DELETE FROM ${table} WHERE org_id='${org}' RETURNING *)
    SELECT coalesce(json_agg(removed), '[]') FROM removed`)]);
  return () => {
    for (const [table, rows] of saved) sql(`INSERT INTO ${table} SELECT * FROM json_populate_recordset(NULL::${table}, $saved$${rows}$saved$::json)`);
  };
};
// Dioxus 0.7.9 appends a build-specific decimal hash to implicit endpoints.
// Read the actual server's registered string; never hard-code a build's hash.
const serverStrings = readFileSync(process.env.HORAE_TEST_SERVER).toString('latin1');
const endpoints = [...new Set(serverStrings.match(/\/api\/save_project_managers\d+/g))];
assert.equal(endpoints.length, 1, 'expected one compiled project-manager endpoint');
const identityEndpoints = [...new Set(serverStrings.match(/\/api\/get_me\d+/g))];
assert.equal(identityEndpoints.length, 1, 'expected one compiled session-identity endpoint');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15_000);
  page.setDefaultNavigationTimeout(30_000);
  const errors = [];
  let releaseRead;
  let actor, project, initialRevision, fixtureStarted = false, restoreSeededStates, restoreEarlierDelegation;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const history = page.getByRole('region', { name: 'Permission changes', exact: true });
  const rows = history.locator('.audit-row');
  const lines = history.locator('.audit-row-line');
  const receiptIds = () => history.locator('.audit-row .text-mono').filter({ hasText: /^Receipt / }).allTextContents();
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    // A preceding suite may create another admin selected by dev login.
    const identity = await page.context().request.post(`${base}${identityEndpoints[0]}`, { data: {} });
    assert.equal(identity.status(), 200, await identity.text());
    actor = await identity.json();
    for (const id of [actor.id, actor.org_id]) assert.match(id, /^[0-9a-f-]{36}$/);
    assert.equal(sql(`SELECT active FROM users WHERE id='${actor.id}' AND org_id='${actor.org_id}'`), 't');
    project = sql(`SELECT id FROM projects WHERE org_id='${actor.org_id}' AND active ORDER BY id LIMIT 1`);
    assert.match(project, /^[0-9a-f-]{36}$/);
    restoreSeededStates = setAsideSeededStates(actor.org_id);
    // Earlier suites designate managers when they create projects, which
    // leaves receipts; history starts empty here and gets them back after.
    restoreEarlierDelegation = setAside(actor.org_id, ['permission_change_receipts', 'project_management_assignments']);
    for (const table of ['person_permission_states', 'permission_change_receipts', 'project_management_assignments']) {
      assert.equal(sql(`SELECT count(*) FROM ${table} WHERE org_id='${actor.org_id}'`), '0');
    }
    initialRevision = sql(`SELECT access_revision FROM organizations WHERE id='${actor.org_id}'`);
    assert.match(initialRevision, /^\d+$/);
    sql(`INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
      VALUES ('${stateId}', '${actor.org_id}', '${actor.id}', 1,
        ARRAY['time_read_own','time_write_own','expense_read_own','expense_write_own','project_read_managed','project_write_managed','project_read_all','project_write_all'], true, 'individual')`);
    fixtureStarted = true;
    await page.goto(`${base}/admin/audit`);
    // The empty log has its own icon, offers People and no paging.
    await expect(page.getByRole('heading', { name: 'No permission changes yet', exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Go to People', exact: true })).toHaveAttribute('href', '/people');
    await expect(page.locator('#audit-load-more')).toHaveCount(0);
    for (const gone of ['Refresh history', 'Newest events', 'Older events']) await expect(page.getByText(gone)).toHaveCount(0);

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
    const projectName = sql(`SELECT name FROM projects WHERE id='${project}'`);
    await page.reload();
    await expect(rows).toHaveCount(25);
    assert.deepEqual(await receiptIds(), ids.slice(0, 25).map(id => `Receipt ${id}`));
    // Each row is a sentence; the newest is the no-op, then the last addition.
    await expect(lines.nth(0)).toContainText(`saved the managers of ${projectName} without changes`);
    await expect(lines.nth(1)).toContainText(`made ${actor.name} a manager of ${projectName}`);
    await expect(lines.nth(2)).toContainText(`removed ${actor.name} as a manager of ${projectName}`);
    await expect(lines.nth(1)).toContainText('Managers');
    await expect(lines.nth(1)).toContainText('+1');
    // Times are local and grouped under a day heading.
    await expect(history.locator('h2.audit-day').first()).toHaveText('Today');
    await expect(page.getByText(/Times in your time zone \(UTC\)\./)).toBeVisible();
    // Names link: the person to the log filtered to them, the project to its page.
    await expect(rows.nth(1).getByRole('link', { name: projectName, exact: true })).toHaveAttribute('href', `/projects/${project}`);
    await expect(rows.nth(1).getByRole('link', { name: actor.name, exact: true }).first()).toHaveAttribute('href', `/admin/audit?person=${actor.id}`);
    // The links and the toggle are separate focus stops, in reading order:
    // actor, person, project, then the toggle, which opens the detail.
    const toggle = page.locator(`#audit-toggle-${ids[1]}`);
    const detail = page.locator(`#audit-detail-${ids[1]}`);
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await expect(toggle).toHaveAttribute('aria-controls', `audit-detail-${ids[1]}`);
    await expect(detail).toBeHidden();
    await rows.nth(1).getByRole('link').first().focus();
    const stops = [];
    for (let stop = 0; stop < 4; stop++) {
      stops.push(await page.evaluate(() => {
        const el = document.activeElement;
        return el.id || `${el.tagName.toLowerCase()}:${el.textContent.trim()}`;
      }));
      await page.keyboard.press('Tab');
    }
    assert.deepEqual(stops, [`a:${actor.name}`, `a:${actor.name}`, `a:${projectName}`, `audit-toggle-${ids[1]}`]);
    await toggle.focus();
    await page.keyboard.press('Enter');
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await expect(detail).toBeVisible();
    await expect(detail).toContainText(`${actor.name} became a manager`);
    await expect(detail).toContainText(`Receipt ${ids[1]}`);

    for (const [width, height, theme] of [[1440, 900, 'dark'], [390, 844, 'light']]) {
      await page.setViewportSize({ width, height });
      await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, 'history must not widen the viewport');
      for (const id of ['audit-kind-menu-trigger', 'audit-person-menu-trigger', 'audit-period-menu-trigger', 'audit-load-more']) {
        const box = await page.locator(`#${id}`).boundingBox();
        assert.ok(box && box.x >= 0 && box.x + box.width <= width + 1, `${id} must remain within the viewport`);
      }
      if (process.env.HORAE_BROWSER_ARTIFACTS) {
        mkdirSync(process.env.HORAE_BROWSER_ARTIFACTS, { recursive: true });
        await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, `permission-history-${width}-${theme}.png`), animations: 'disabled' });
      }
    }
    await page.setViewportSize({ width: 1440, height: 900 });
    // "Load more" appends the rest and then goes away.
    await page.locator('#audit-load-more').focus();
    await page.keyboard.press('Enter');
    await expect(rows).toHaveCount(28);
    assert.deepEqual(await receiptIds(), ids.map(id => `Receipt ${id}`));
    await expect(page.locator('#audit-load-more')).toHaveCount(0);
    // Another filter, then Back: the log starts again from the newest instead
    // of reusing the cursor of the earlier "Load more".
    await page.locator('#audit-kind-menu-trigger').click();
    await page.getByRole('menuitem', { name: 'Changes to a profile', exact: true }).click();
    await page.waitForURL(`${base}/admin/audit?kind=profile`);
    await expect(page.getByRole('heading', { name: 'No changes match these filters', exact: true })).toBeVisible();
    await page.goBack();
    await page.waitForURL(/\/admin\/audit\??$/);
    await expect(page.locator('#audit-kind-menu-trigger')).toContainText('All changes');
    await expect(page.locator('.audit-content')).not.toHaveAttribute('aria-busy', 'true');
    await expect(rows).toHaveCount(25);
    assert.deepEqual(await receiptIds(), ids.slice(0, 25).map(id => `Receipt ${id}`));

    // Filters live in the address. One that matches nothing offers to clear it.
    await page.goto(`${base}/admin/audit?kind=profile`);
    await expect(page.getByRole('heading', { name: 'No changes match these filters', exact: true })).toBeVisible();
    await expect(page.locator('#audit-kind-menu-trigger')).toContainText('Changes to a profile');
    await page.locator('#audit-clear-filters').click();
    // The router writes the empty query as a bare `?`.
    await page.waitForURL(/\/admin\/audit\??$/);
    await expect(rows).toHaveCount(25);

    let intercepted = false;
    const heldRead = new Promise(resolve => { releaseRead = resolve; });
    const holdRead = async route => { intercepted = true; await heldRead; await route.continue(); };
    await page.route('**/api/list_permission_audit*', holdRead);
    try {
      // A new filter keeps the rows on screen, dimmed, until its answer arrives.
      await page.locator('#audit-kind-menu-trigger').click();
      await page.getByRole('menuitem', { name: 'Changes to project managers', exact: true }).click();
      await expect.poll(() => intercepted).toBe(true);
      await expect(rows).toHaveCount(25);
      await expect(page.locator('.audit-content')).toHaveAttribute('aria-busy', 'true');
    } finally {
      releaseRead();
    }
    await page.waitForURL(`${base}/admin/audit?kind=managers`);
    await expect(page.locator('.audit-content')).not.toHaveAttribute('aria-busy', 'true');
    await expect(rows).toHaveCount(25);
    await page.unroute('**/api/list_permission_audit*', holdRead);
    sql(`UPDATE person_permission_states SET is_administrator=false WHERE id='${stateId}'`);
    await page.reload();
    // A refusal keeps only the header and the one no-access state.
    await expect(page.getByRole('alert')).toContainText('Access unavailable');
    await expect(history).not.toBeVisible();
    for (const id of ['audit-kind-menu-trigger', 'audit-load-more']) await expect(page.locator(`#${id}`)).toHaveCount(0);
    sql(`UPDATE person_permission_states SET is_administrator=true WHERE id='${stateId}'`);
    await page.goto(`${base}/admin/audit`);
    await expect(rows).toHaveCount(25);
    assert.deepEqual(await receiptIds(), ids.slice(0, 25).map(id => `Receipt ${id}`));

    await page.goto(`${base}/settings/access`);
    await page.getByRole('link', { name: 'See changes to your access', exact: true }).click();
    await page.waitForURL(`${base}/admin/audit?person=${actor.id}`);
    await expect(rows).toHaveCount(25);
    sql(`UPDATE users SET active=false WHERE id='${actor.id}'`);
    await page.locator('#audit-period-menu-trigger').click();
    await page.getByRole('menuitem', { name: 'Last 30 days', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('Sign in again');
    await expect(history).not.toBeVisible();
    assert.deepEqual(errors, []);
    console.log(`PASS: permission history in Chromium ${browser.version()}: real writer receipts as sentences by day, empty and filtered-empty states, load more, keyboard details, filters in the URL, dimmed reloads, responsive themes, revocation, canonical admin navigation and deactivation`);
  } finally {
    releaseRead?.();
    await browser.close();
    if (fixtureStarted) sql(`BEGIN;
      UPDATE users SET active=true WHERE id='${actor.id}';
      DELETE FROM project_management_assignments WHERE org_id='${actor.org_id}' AND manager_id='${actor.id}' AND project_id='${project}';
      DELETE FROM permission_change_receipts WHERE org_id='${actor.org_id}' AND actor_user_id='${actor.id}' AND request_id::text LIKE '01960000-0000-7000-8000-%';
      DELETE FROM person_permission_states WHERE id='${stateId}' AND org_id='${actor.org_id}';
      UPDATE organizations SET access_revision=${initialRevision} WHERE id='${actor.org_id}'; COMMIT;`);
    restoreSeededStates?.();
    restoreEarlierDelegation?.();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
