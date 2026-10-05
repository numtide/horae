// Real report reads and downloads, exclusively in the runner's disposable DB.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const { mkdirSync } = require('node:fs');
const { join } = require('node:path');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id FROM users WHERE email='admin@example.com' AND active AND org_role='admin') u"));
const org = actor.org_id;
const id = n => `019f3000-0000-7000-8000-${String(n).padStart(12, '0')}`;
const person = id(1), client = id(2), project = id(3), task = id(4), state = id(5);
const floor = "ARRAY['time_read_own','time_write_own','expense_read_own','expense_write_own']";
const date = '2040-01-02';
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${org}'`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [], requests = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('request', request => requests.push(new URL(request.url()).pathname));
  let release;
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: /Sign in as Admin/ }).click();
    await page.waitForURL(url => !url.pathname.startsWith('/auth/'));
    await page.goto(`${base}/reports`);
    await expect(page.getByRole('button', { name: 'Time', exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Export CSV', exact: true })).toBeVisible();

    sql(`BEGIN;
      INSERT INTO users (id,org_id,email,name) VALUES ('${person}','${org}','report-fixture@example.test','Private colleague');
      INSERT INTO clients (id,org_id,name,currency) VALUES ('${client}','${org}','Report fixture client','EUR');
      INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ('${project}','${org}','${client}','Report fixture project','EUR');
      INSERT INTO tasks (id,org_id,name) VALUES ('${task}','${org}','Report fixture task');
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,notes)
        SELECT ('019f3000-0000-7000-8000-' || lpad((1000+n)::text,12,'0'))::uuid,'${org}','${actor.id}','${project}','${task}','${date}',60,true,'<Report & note> ' || n FROM generate_series(1,503) n;
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,notes)
        VALUES ('${id(6)}','${org}','${person}','${project}','${task}','${date}',120,false,'Private report note');
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('${state}','${org}','${actor.id}',1,${floor},false,'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);
    requests.length = 0;
    await page.goto(`${base}/reports`);
    await expect(page.locator('#report-refresh')).toBeEnabled();
    await page.locator('#report-to').fill(date);
    await expect(page.locator('#report-refresh')).toBeEnabled();
    await page.locator('#report-from').fill(date);
    await expect(page.locator('tbody tr')).toHaveCount(500);
    await expect(page.locator('dl')).toContainText('503.00');
    await expect(page.locator('tbody')).not.toContainText('Private report note');
    assert.equal(requests.filter(path => /\/api\/(list_clients|list_users|list_project_tags|list_projects|report_time|report_detailed)(\b|_)/.test(path)).length, 0, 'canonical report must not mount legacy resources');
    await expect(page.locator('tbody')).toContainText('<Report & note>');
    const csvLink = await page.getByRole('link', { name: 'Export CSV', exact: true }).getAttribute('href');
    const xlsxLink = await page.getByRole('link', { name: 'Export XLSX', exact: true }).getAttribute('href');
    for (const link of [csvLink, xlsxLink]) {
      assert.ok(link.includes('expected_policy=scoped') && link.includes(`expected_user_id=${actor.id}`));
      assert.ok(!link.includes('after='));
    }
    const csv = await context.request.get(`${base}${csvLink}`);
    assert.equal(csv.status(), 200);
    const csvText = await csv.text();
    assert.equal(csvText.match(/<Report & note>/g).length, 503);
    assert.ok(!csvText.includes('Private report note'));
    const xlsx = await context.request.get(`${base}${xlsxLink}`);
    assert.equal(xlsx.status(), 200);
    assert.ok((await xlsx.body()).subarray(0, 2).equals(Buffer.from('PK')));

    await page.locator('#report-next').click();
    await expect(page.locator('tbody tr')).toHaveCount(3);
    await expect(page.locator('dl')).toContainText('503.00');
    await expect(page.locator('#report-next')).toBeDisabled();
    await page.locator('#report-previous').click();
    await expect(page.locator('tbody tr')).toHaveCount(500);
    await page.locator('#report-from').fill('2040-01-03');
    await expect(page.locator('#report-date-error')).toBeVisible();
    await expect(page.locator('tbody')).toHaveCount(0);
    await expect(page.getByRole('link', { name: 'Export CSV', exact: true })).toHaveCount(0);
    await page.locator('#report-from').fill(date);
    await expect(page.locator('tbody tr')).toHaveCount(500);

    let held = false;
    await page.route('**/api/list_visible_time_report_entries*', async route => {
      held = true;
      await new Promise(resolve => { release = resolve; });
      await route.continue();
    }, { times: 1 });
    await page.locator('#report-refresh').click();
    await expect.poll(() => held).toBe(true);
    await expect(page.locator('tbody')).toHaveCount(0);
    await expect(page.locator('dl')).toHaveCount(0);
    await expect(page.getByRole('link', { name: 'Export CSV', exact: true })).toHaveCount(0);
    release();
    release = undefined;
    await expect(page.locator('tbody tr')).toHaveCount(500);

    const filteredLinks = [];
    for (const [dimension, entity, filter] of [
      ['client', client, 'client_ids'], ['project', project, 'project_ids'],
      ['task', task, 'task_ids'], ['person', actor.id, 'user_ids'],
    ]) {
      await page.locator('#report-view-time').click();
      await page.locator(`#report-group-${dimension}`).click();
      await expect(page.locator(`#report-hours-${entity}`)).toHaveText('503.00');
      await expect(page.locator('tbody tr')).toHaveCount(1);
      await expect(page.locator('tbody tr td:nth-child(3)')).toHaveText('503.00');
      await expect(page.locator('tbody tr td:nth-child(4)')).toHaveText('0.00');
      await expect(page.locator('tbody')).not.toContainText('Private colleague');
      await expect(page.getByRole('link', { name: 'Export CSV', exact: true })).toHaveCount(0);
      await page.locator(`#report-hours-${entity}`).focus();
      await page.keyboard.press('Enter');
      await expect(page.locator('tbody tr')).toHaveCount(500);
      for (const format of ['CSV', 'XLSX']) {
        const link = await page.getByRole('link', { name: `Export ${format}`, exact: true }).getAttribute('href');
        const params = new URL(link, base).searchParams;
        assert.equal(params.get(filter), entity);
        assert.equal(params.get('from'), date);
        assert.equal(params.get('to'), date);
        assert.equal(params.get('expected_user_id'), actor.id);
        assert.equal(params.get('expected_policy'), 'scoped');
        for (const other of ['client_ids', 'project_ids', 'task_ids', 'user_ids'].filter(key => key !== filter)) {
          assert.equal(params.has(other), false, 'a previous grouping must not leak into the drilldown');
        }
        const response = await context.request.get(`${base}${link}`);
        assert.equal(response.status(), 200);
        if (format === 'CSV') {
          const body = await response.text();
          assert.equal(body.match(/<Report & note>/g).length, 503);
          assert.ok(!body.includes('Private report note'));
        } else {
          assert.ok((await response.body()).subarray(0, 2).equals(Buffer.from('PK')));
        }
        filteredLinks.push(link);
      }
      await page.locator('#report-clear-selection').click();
      await expect(page.locator('tbody tr')).toHaveCount(500);
      const cleared = new URL(await page.getByRole('link', { name: 'Export CSV', exact: true }).getAttribute('href'), base);
      assert.equal(cleared.searchParams.has(filter), false);
    }
    await page.locator('#report-view-time').click();
    await expect(page.locator(`#report-hours-${actor.id}`)).toBeVisible();
    await page.locator('#report-from').fill('2040-01-01');
    await expect(page.locator('#report-group-refresh')).toBeEnabled();
    await page.locator('#report-to').fill('2040-01-01');
    await expect(page.locator('#report-group-refresh')).toBeEnabled();
    await expect(page.locator('tbody')).toHaveCount(0);
    await expect(page.locator('dl')).toContainText('0.00');
    await expect(page.locator('#report-group-person')).toHaveAttribute('aria-pressed', 'true');
    await page.locator('#report-to').fill(date);
    await expect(page.locator(`#report-hours-${actor.id}`)).toHaveText('503.00');
    await page.locator('#report-from').fill(date);
    await expect(page.locator('#report-group-refresh')).toBeEnabled();
    await page.locator('#report-view-detailed').click();
    await expect(page.locator('tbody tr')).toHaveCount(500);

    for (const [width, height, theme] of [[1440, 900, 'dark'], [390, 844, 'light']]) {
      await page.setViewportSize({ width, height });
      await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
      await page.evaluate(() => new Promise(requestAnimationFrame));
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false, 'viewport must not overflow');
      assert.equal(await page.locator('#report-from').evaluate(el => el.labels.length), 1);
      for (const [column, characters] of [[1, 10], [2, 6], [3, 6]]) {
        const lines = await page.locator(`tbody tr:first-child td:nth-child(${column})`).evaluate((cell, length) => {
          const range = document.createRange();
          range.setStart(cell.firstChild, 0);
          range.setEnd(cell.firstChild, length);
          return range.getClientRects().length;
        }, characters);
        assert.equal(lines, 1, 'dates and individual words must not break within a table cell');
      }
      await page.locator('#report-refresh').focus();
      await expect(page.locator('#report-refresh')).toBeFocused();
      assert.ok(await page.locator('#report-from').evaluate(el => getComputedStyle(el).getPropertyValue('--color-bg').trim()), 'bundle must include theme tokens');
      if (process.env.HORAE_BROWSER_ARTIFACTS) {
        mkdirSync(process.env.HORAE_BROWSER_ARTIFACTS, { recursive: true });
        await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, `reports-${width}-${theme}.png`) });
      }
      await page.locator('#report-view-time').click();
      await page.locator('#report-group-project').click();
      await expect(page.locator(`#report-hours-${project}`)).toHaveText('503.00');
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false, 'grouped viewport must not overflow');
      const nameLines = await page.locator('tbody tr:first-child td:first-child').evaluate(cell => {
        const range = document.createRange();
        range.setStart(cell.firstChild, 0);
        range.setEnd(cell.firstChild, 6);
        return range.getClientRects().length;
      });
      assert.equal(nameLines, 1, 'group names must not break within individual words');
      await page.locator(`#report-hours-${project}`).focus();
      await expect(page.locator(`#report-hours-${project}`)).toBeFocused();
      if (process.env.HORAE_BROWSER_ARTIFACTS) {
        await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, `report-groups-${width}-${theme}.png`) });
      }
      await page.locator('#report-view-detailed').click();
      await expect(page.locator('tbody tr')).toHaveCount(500);
    }
    await page.locator('#report-view-time').click();
    await expect(page.locator(`#report-hours-${project}`)).toBeVisible();
    // No grant change is needed: even an Admin's captured scoped link must not
    // adopt the wider policy-0 scope after the mode changes.
    sql(`UPDATE organizations SET permission_policy_version=0 WHERE id='${org}'`);
    for (const link of [csvLink, xlsxLink, ...filteredLinks]) {
      assert.equal((await context.request.get(`${base}${link}`)).status(), 403);
    }
    await page.locator('#report-group-refresh').click();
    await expect(page.getByRole('alert')).toContainText('Could not load the report');
    await expect(page.locator('tbody')).toHaveCount(0);
    await page.locator('#reports-retry-access').click();
    await expect(page.getByRole('alert')).toContainText('session or report policy changed');
    await expect(page.locator('tbody')).toHaveCount(0);
    sql(`UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'`);
    await page.locator('#reports-retry-access').click();
    await expect(page.locator('#report-refresh')).toBeEnabled();
    sql(`UPDATE users SET active=false WHERE id='${actor.id}'`);
    await page.locator('#report-refresh').click();
    await expect(page.getByRole('alert')).toContainText('Could not load the report');
    await expect(page.locator('tbody')).toHaveCount(0);
    await expect(page.getByRole('link', { name: 'Export CSV', exact: true })).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log(`PASS: scoped report paging, totals, four group drilldowns, filtered exports, stale/invalid states and policy binding; Chromium ${browser.version()}`);
  } finally {
    if (release) release();
    await page.unrouteAll({ behavior: 'wait' });
    await browser.close();
    sql(`BEGIN;
      UPDATE users SET active=true WHERE id='${actor.id}';
      UPDATE organizations SET permission_policy_version=0 WHERE id='${org}';
      DELETE FROM person_permission_states WHERE id='${state}';
      DELETE FROM time_entries WHERE project_id='${project}';
      DELETE FROM projects WHERE id='${project}';
      DELETE FROM clients WHERE id='${client}';
      DELETE FROM tasks WHERE id='${task}';
      DELETE FROM users WHERE id='${person}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
