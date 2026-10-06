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
      INSERT INTO project_tags (id,org_id,name) VALUES ('${id(7)}','${org}','Report export tag');
      INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ('${id(8)}','${org}','${project}','${id(7)}');
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
      await expect(page.getByRole('link', { name: 'Export CSV', exact: true })).toBeVisible();
      const groupedLink = await page.getByRole('link', { name: 'Export XLSX', exact: true }).getAttribute('href');
      const groupedUrl = new URL(groupedLink, base);
      assert.equal(groupedUrl.pathname, '/api/reports/time/grouped/xlsx');
      assert.equal(groupedUrl.searchParams.get('group_by'), dimension);
      assert.equal(groupedUrl.searchParams.get('expected_user_id'), actor.id);
      assert.equal(groupedUrl.searchParams.get('expected_policy'), 'scoped');
      assert.equal(groupedUrl.searchParams.has('after'), false);
      const workbook = await context.request.get(groupedUrl.href);
      assert.equal(workbook.status(), 200);
      assert.ok((await workbook.body()).subarray(0, 2).equals(Buffer.from('PK')));
      filteredLinks.push(groupedLink);
      const groupedCsv = new URL(await page.getByRole('link', { name: 'Export CSV', exact: true }).getAttribute('href'), base);
      assert.equal(groupedCsv.pathname, '/api/reports/time/grouped/csv');
      assert.equal(groupedCsv.search, groupedUrl.search);
      const download = await context.request.get(groupedCsv.href);
      assert.equal(download.status(), 200);
      assert.equal(download.headers()['content-type'], 'text/csv');
      assert.equal(download.headers()['content-disposition'], 'attachment; filename="time-report.csv"');
      const groupedText = await download.text();
      assert.equal(groupedText.split('\n').length, 3);
      assert.ok(groupedText.endsWith(',503.00,503.00,0.00\n'));
      assert.ok(!groupedText.includes('Private colleague') && !groupedText.includes(actor.id));
      filteredLinks.push(`${groupedCsv.pathname}${groupedCsv.search}`);
      const narrowed = new URL(groupedCsv.href);
      for (const [field, value] of [['client_ids', client], ['project_ids', project], ['user_ids', actor.id], ['task_ids', task], ['tag_ids', id(7)]]) {
        narrowed.searchParams.set(field, `${value},${id(99)},${value}`);
      }
      const filteredDownload = await context.request.get(narrowed.href);
      assert.equal(filteredDownload.status(), 200);
      assert.equal(await filteredDownload.text(), groupedText);
      for (const field of ['client_ids', 'project_ids', 'user_ids', 'task_ids', 'tag_ids']) {
        const empty = new URL(narrowed.href);
        empty.searchParams.set(field, id(99));
        const result = await context.request.get(empty.href);
        assert.equal(result.status(), 200);
        assert.equal(await result.text(), `${groupedText.split('\n')[0]}\n`);
      }
      if (dimension === 'client') {
        for (const suffix of ['&group_by=person', '&from=2040-01-01', '&after=', '&user_ids=invalid']) {
          assert.equal((await context.request.get(`${groupedUrl.href}${suffix}`)).status(), 400);
          assert.equal((await context.request.get(`${groupedCsv.href}${suffix}`)).status(), 400);
        }
        const mismatched = new URL(groupedUrl.href);
        mismatched.searchParams.set('expected_user_id', person);
        assert.equal((await context.request.get(mismatched.href)).status(), 403);
        mismatched.pathname = groupedCsv.pathname;
        assert.equal((await context.request.get(mismatched.href)).status(), 403);
        const anonymous = await browser.newContext();
        try {
          assert.equal((await anonymous.request.get(groupedUrl.href)).status(), 401);
          assert.equal((await anonymous.request.get(groupedCsv.href)).status(), 401);
        } finally {
          await anonymous.close();
        }
      }
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
    const roots = [
      ['client', client, 'client_ids', ['project', 'task', 'person']],
      ['project', project, 'project_ids', ['task', 'person']],
      ['task', task, 'task_ids', ['project', 'person']],
      ['person', actor.id, 'user_ids', ['project', 'task']],
    ];
    const openIndividual = async (dimension, entity) => {
      await page.locator('#report-view-time').click();
      await page.locator(`#report-group-${dimension}`).click();
      await expect(page.locator(`#report-name-${entity}`)).toBeVisible();
      await page.locator(`#report-name-${entity}`).focus();
      await page.keyboard.press('Enter');
      await expect(page.locator('#report-group-root')).toBeVisible();
      await expect(page.locator('#report-group-detail')).toBeVisible();
    };
    const checkDownloads = async (filters, grouped) => {
      for (const format of ['CSV', 'XLSX']) {
        for (const [filter, entity] of Object.entries(filters)) {
          await expect(page.getByRole('link', { name: `Export ${format}`, exact: true })).toHaveAttribute('href', new RegExp(`${filter}=${entity}`));
        }
        const link = await page.getByRole('link', { name: `Export ${format}`, exact: true }).getAttribute('href');
        const url = new URL(link, base);
        assert.equal(url.pathname, grouped ? `/api/reports/time/grouped/${format.toLowerCase()}` : `/api/reports/export/${format.toLowerCase()}`);
        for (const filter of ['client_ids', 'project_ids', 'task_ids', 'user_ids']) {
          assert.equal(url.searchParams.get(filter), filters[filter] || null);
        }
        assert.equal(url.searchParams.get('from'), date);
        assert.equal(url.searchParams.get('to'), date);
        assert.equal(url.searchParams.get('expected_user_id'), actor.id);
        assert.equal(url.searchParams.get('expected_policy'), 'scoped');
        assert.equal(url.searchParams.has('after'), false);
        const response = await context.request.get(url.href);
        assert.equal(response.status(), 200);
        if (format === 'CSV') {
          const body = await response.text();
          assert.ok(!body.includes('Private colleague') && !body.includes('Private report note'));
          if (grouped) assert.ok(body.endsWith(',503.00,503.00,0.00\n'));
          else assert.equal(body.match(/<Report & note>/g).length, 503);
        } else assert.ok((await response.body()).subarray(0, 2).equals(Buffer.from('PK')));
        filteredLinks.push(link);
      }
    };
    for (const [dimension, entity, filter, tabs] of roots) {
      await openIndividual(dimension, entity);
      for (const tab of ['client', 'project', 'task', 'person']) {
        await expect(page.locator(`#report-group-${tab}`)).toHaveCount(tabs.includes(tab) ? 1 : 0);
      }
      await checkDownloads({ [filter]: entity }, true);
      await page.locator('#report-group-detail').click();
      await expect(page.locator('tbody tr')).toHaveCount(500);
      await checkDownloads({ [filter]: entity }, false);
      if (dimension !== 'client' && dimension !== 'project') continue;
      for (const [tab, row, leaf, leafFilter] of [
        ['task', task, actor.id, 'user_ids'],
        ['person', actor.id, dimension === 'client' ? project : task, dimension === 'client' ? 'project_ids' : 'task_ids'],
      ]) {
        await openIndividual(dimension, entity);
        await page.locator(`#report-group-${tab}`).click();
        await expect(page.locator(`#report-expand-${row}`)).toHaveAttribute('aria-expanded', 'false');
        await page.locator(`#report-expand-${row}`).focus();
        await page.keyboard.press('Enter');
        await expect(page.locator(`#report-expanded-hours-${leaf}`)).toHaveText('503.00');
        await expect(page.locator(`#report-expand-${row}`)).toHaveAttribute('aria-expanded', 'true');
        await expect(page.locator(`#report-breakdown-${row}`)).not.toContainText('Private colleague');
        await page.locator(`#report-expanded-hours-${leaf}`).click();
        await expect(page.locator('tbody tr')).toHaveCount(500);
        await checkDownloads({ [filter]: entity, [tab === 'task' ? 'task_ids' : 'user_ids']: row, [leafFilter]: leaf }, false);
      }
    }
    await openIndividual('client', client);
    await page.locator(`#report-name-${project}`).click();
    await expect(page.locator('#report-group-detail')).toBeVisible();
    await checkDownloads({ project_ids: project }, true);
    await page.locator('#report-group-root').click();
    await page.locator('#report-group-person').click();
    await expect(page.locator(`#report-hours-${actor.id}`)).toBeVisible();
    await page.locator('#report-from').fill('2040-01-01');
    await expect(page.locator('#report-group-refresh')).toBeEnabled();
    await page.locator('#report-to').fill('2040-01-01');
    await expect(page.locator('#report-group-refresh')).toBeEnabled();
    await expect(page.locator('tbody')).toHaveCount(0);
    await expect(page.locator('dl')).toContainText('0.00');
    const emptyWorkbook = await context.request.get(new URL(await page.getByRole('link', { name: 'Export XLSX', exact: true }).getAttribute('href'), base).href);
    assert.equal(emptyWorkbook.status(), 200);
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
      assert.equal(await page.locator('#report-billability').evaluate(el => el.labels.length), 1);
      await page.getByRole('combobox', { name: 'Show', exact: true }).focus();
      await expect(page.locator('#report-billability')).toBeFocused();
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
        await page.screenshot({ animations: 'disabled', path: join(process.env.HORAE_BROWSER_ARTIFACTS, `reports-${width}-${theme}.png`) });
      }
      await page.locator('#report-view-time').click();
      await page.locator('#report-group-project').click();
      await expect(page.locator(`#report-hours-${project}`)).toHaveText('503.00');
      await expect(page.locator('[aria-label="Group time by"] [aria-pressed="true"]')).toHaveCount(1);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false, 'grouped viewport must not overflow');
      const nameLines = await page.locator(`#report-name-${project}`).evaluate(button => {
        const range = document.createRange();
        range.setStart(button.firstChild, 0);
        range.setEnd(button.firstChild, 6);
        return range.getClientRects().length;
      });
      assert.equal(nameLines, 1, 'group names must not break within individual words');
      await page.locator(`#report-hours-${project}`).focus();
      await expect(page.locator(`#report-hours-${project}`)).toBeFocused();
      if (process.env.HORAE_BROWSER_ARTIFACTS) {
        await page.screenshot({ animations: 'disabled', path: join(process.env.HORAE_BROWSER_ARTIFACTS, `report-groups-${width}-${theme}.png`) });
      }
      await page.locator(`#report-name-${project}`).click();
      await expect(page.locator(`#report-expand-${task}`)).toBeVisible();
      await page.locator(`#report-expand-${task}`).click();
      await expect(page.locator(`#report-expanded-hours-${actor.id}`)).toHaveText('503.00');
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false, 'nested viewport must not overflow');
      await page.locator(`#report-expand-${task}`).focus();
      await expect(page.locator(`#report-expand-${task}`)).toBeFocused();
      if (process.env.HORAE_BROWSER_ARTIFACTS) {
        await page.screenshot({ fullPage: true, animations: 'disabled', path: join(process.env.HORAE_BROWSER_ARTIFACTS, `report-nested-${width}-${theme}.png`) });
      }
      await page.locator('#report-view-detailed').click();
      await expect(page.locator('tbody tr')).toHaveCount(500);
    }
    sql(`BEGIN;
      UPDATE projects SET active=false WHERE id='${project}';
      INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ('${id(9)}','${org}','${client}','Active report project','EUR');
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,notes) VALUES
        ('${id(10)}','${org}','${actor.id}','${id(9)}','${task}','${date}',30,true,'Active own entry'),
        ('${id(11)}','${org}','${person}','${id(9)}','${task}','${date}',180,true,'Private active entry');
      COMMIT;`);
    await page.locator('#report-refresh').click();
    await expect(page.locator('dl')).toContainText('503.50');
    const activeOnly = page.getByRole('checkbox', { name: 'Active projects only', exact: true });
    await expect(activeOnly).toHaveAttribute('aria-checked', 'false');
    await page.locator('#report-next').click();
    await expect(page.locator('tbody tr')).toHaveCount(4);
    await activeOnly.focus();
    await page.keyboard.press('Space');
    await expect(activeOnly).toHaveAttribute('aria-checked', 'true');
    await expect(page.locator('tbody tr')).toHaveCount(1);
    await expect(page.locator('tbody')).toContainText('Active own entry');
    await expect(page.locator('#report-previous')).toBeDisabled();
    await expect(page.locator('dl')).toContainText('0.50');
    const checkActiveDownload = async grouped => {
      for (const format of ['CSV', 'XLSX']) {
        const link = page.getByRole('link', { name: `Export ${format}`, exact: true });
        await expect(link).toHaveAttribute('href', /active_projects_only=true/);
        const url = new URL(await link.getAttribute('href'), base);
        const result = await context.request.get(url.href);
        assert.equal(result.status(), 200);
        if (format === 'CSV') {
          const text = await result.text();
          assert.ok(!text.includes('Private active entry') && !text.includes('<Report & note>'));
          if (grouped) assert.ok(text.endsWith(',0.50,0.50,0.00\n'));
          else assert.equal(text.match(/Active own entry/g).length, 1);
        } else assert.ok((await result.body()).subarray(0, 2).equals(Buffer.from('PK')));
        for (const invalid of ['', '1', 'yes', 'null']) {
          const bad = new URL(url);
          bad.searchParams.set('active_projects_only', invalid);
          assert.equal((await context.request.get(bad.href)).status(), 400);
        }
        assert.equal((await context.request.get(`${url.href}&active_projects_only=false`)).status(), 400);
        filteredLinks.push(`${url.pathname}${url.search}`);
      }
    };
    await checkActiveDownload(false);
    await page.locator('#report-view-time').click();
    for (const [dimension, entity] of [['client', client], ['project', id(9)], ['task', task], ['person', actor.id]]) {
      await page.locator(`#report-group-${dimension}`).click();
      await expect(page.locator(`#report-hours-${entity}`)).toHaveText('0.50');
      await checkActiveDownload(true);
    }
    await page.locator('#report-group-project').click();
    await page.locator(`#report-name-${id(9)}`).click();
    await expect(page.locator(`#report-expand-${task}`)).toBeVisible();
    await page.locator(`#report-expand-${task}`).click();
    await expect(page.locator(`#report-expanded-hours-${actor.id}`)).toHaveText('0.50');
    await page.locator(`#report-expanded-hours-${actor.id}`).click();
    await expect(page.locator('tbody tr')).toHaveCount(1);
    await checkActiveDownload(false);
    await page.locator('#report-clear-selection').click();
    await expect(page.locator('#report-refresh')).toBeEnabled();
    sql(`UPDATE projects SET active=false WHERE id='${id(9)}'`);
    await page.locator('#report-refresh').click();
    await expect(page.locator('tbody')).toHaveCount(0);
    await expect(page.locator('dl')).toContainText('0.00');
    const emptyCsv = await context.request.get(new URL(await page.getByRole('link', { name: 'Export CSV', exact: true }).getAttribute('href'), base).href);
    assert.equal(emptyCsv.status(), 200);
    assert.equal((await emptyCsv.text()).trim().split('\n').length, 1);
    await activeOnly.click();
    await expect(page.locator('dl')).toContainText('503.50');
    // Effective billability changes without changing any entry's raw flag.
    sql(`UPDATE projects SET project_type='non_billable' WHERE id='${project}'`);
    const billability = page.getByRole('combobox', { name: 'Show', exact: true });
    await expect(billability).toHaveValue('all');
    await billability.focus();
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await expect(billability).toHaveValue('billable');
    await expect(page.locator('tbody tr')).toHaveCount(1);
    await expect(page.locator('tbody')).toContainText('Active own entry');
    const checkBillabilityDownload = async (grouped, filter, hours) => {
      for (const format of ['CSV', 'XLSX']) {
        const link = page.getByRole('link', { name: `Export ${format}`, exact: true });
        await expect(link).toHaveAttribute('href', new RegExp(`billability=${filter}`));
        const url = new URL(await link.getAttribute('href'), base);
        assert.equal(url.searchParams.get('expected_user_id'), actor.id);
        assert.equal(url.searchParams.get('expected_policy'), 'scoped');
        const response = await context.request.get(url.href);
        assert.equal(response.status(), 200);
        if (format === 'CSV') {
          const text = await response.text();
          assert.ok(!text.includes('Private report note') && !text.includes('Private active entry'));
          if (grouped) {
            const billable = filter === 'billable' ? hours : '0.00';
            const nonBillable = filter === 'non_billable' ? hours : '0.00';
            assert.ok(text.endsWith(`,${hours},${billable},${nonBillable}\n`));
          } else {
            assert.equal(text.includes('Active own entry'), filter === 'billable');
            assert.equal(text.includes('<Report & note>'), filter === 'non_billable');
          }
        } else assert.ok((await response.body()).subarray(0, 2).equals(Buffer.from('PK')));
        for (const invalid of ['', 'true', 'null', 'unknown']) {
          const bad = new URL(url);
          bad.searchParams.set('billability', invalid);
          assert.equal((await context.request.get(bad.href)).status(), 400);
        }
        assert.equal((await context.request.get(`${url.href}&billability=all`)).status(), 400);
        filteredLinks.push(`${url.pathname}${url.search}`);
      }
    };
    await checkBillabilityDownload(false, 'billable', '0.50');
    await billability.selectOption('non_billable');
    await expect(page.locator('tbody tr')).toHaveCount(500);
    await expect(page.locator('dl')).toContainText('503.00');
    await checkBillabilityDownload(false, 'non_billable', '503.00');
    await page.locator('#report-next').click();
    await expect(page.locator('tbody tr')).toHaveCount(3);
    await billability.selectOption('billable');
    await expect(page.locator('tbody tr')).toHaveCount(1);
    await expect(page.locator('#report-previous')).toBeDisabled();
    await page.locator('#report-view-time').click();
    for (const [dimension, entity] of [['client', client], ['project', id(9)], ['task', task], ['person', actor.id]]) {
      await page.locator(`#report-group-${dimension}`).click();
      await expect(page.locator(`#report-hours-${entity}`)).toHaveText('0.50');
      await checkBillabilityDownload(true, 'billable', '0.50');
    }
    await billability.selectOption('non_billable');
    await expect(page.locator(`#report-hours-${actor.id}`)).toHaveText('503.00');
    await checkBillabilityDownload(true, 'non_billable', '503.00');
    await page.locator('#report-group-project').click();
    await page.locator(`#report-name-${project}`).click();
    await page.locator(`#report-expand-${task}`).click();
    await expect(page.locator(`#report-expanded-hours-${actor.id}`)).toHaveText('503.00');
    await page.locator(`#report-expanded-hours-${actor.id}`).click();
    await expect(page.locator('tbody tr')).toHaveCount(500);
    await expect(billability).toHaveValue('non_billable');
    await checkBillabilityDownload(false, 'non_billable', '503.00');
    await billability.selectOption('billable');
    await expect(page.locator('tbody')).toHaveCount(0);
    await expect(page.locator('dl')).toContainText('0.00');
    const noBillable = await context.request.get(new URL(await page.getByRole('link', { name: 'Export CSV', exact: true }).getAttribute('href'), base).href);
    assert.equal(noBillable.status(), 200);
    assert.equal((await noBillable.text()).trim().split('\n').length, 1);
    await page.locator('#report-clear-selection').click();
    await expect(page.locator('tbody tr')).toHaveCount(1);
    await billability.selectOption('all');
    await expect(page.locator('dl')).toContainText('503.50');
    await page.locator('#report-view-time').click();
    await page.locator('#report-group-project').click();
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
    console.log(`PASS: scoped report paging, totals, four individual reports, four nested breakdowns, filtered exports, stale/invalid states and policy binding; Chromium ${browser.version()}`);
  } finally {
    if (release) release();
    await page.unrouteAll({ behavior: 'wait' });
    await browser.close();
    sql(`BEGIN;
      UPDATE users SET active=true WHERE id='${actor.id}';
      UPDATE organizations SET permission_policy_version=0 WHERE id='${org}';
      DELETE FROM person_permission_states WHERE id='${state}';
      DELETE FROM time_entries WHERE project_id IN ('${project}','${id(9)}');
      DELETE FROM project_tag_links WHERE id='${id(8)}';
      DELETE FROM project_tags WHERE id='${id(7)}';
      DELETE FROM projects WHERE id IN ('${project}','${id(9)}');
      DELETE FROM clients WHERE id='${client}';
      DELETE FROM tasks WHERE id='${task}';
      DELETE FROM users WHERE id='${person}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
