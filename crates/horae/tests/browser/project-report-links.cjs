// Every record and role change belongs to the runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.ok(database.pathname === '/horae' && database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const project = sql("SELECT id FROM projects WHERE code='ACME-01'");
const task = sql("SELECT id FROM tasks WHERE name='Development'");
const admin = sql("SELECT id FROM users WHERE email='admin@example.com'");
const sortingProject = '01970000-0000-7000-8000-000000000701';
const sortingPerson = '01970000-0000-7000-8000-000000000705';
const person = JSON.parse(sql(`SELECT json_build_object('id', u.id, 'name', u.name) FROM time_entries te JOIN users u ON u.id=te.user_id WHERE te.project_id='${project}' AND te.task_id='${task}' ORDER BY te.spent_date LIMIT 1`));
const day = sql(`SELECT min(spent_date) FROM time_entries WHERE project_id='${project}' AND task_id='${task}' AND user_id='${person.id}'`);
const count = (taskFilter, personFilter, custom) => Number(sql(`SELECT count(*) FROM time_entries WHERE project_id='${project}'${taskFilter ? ` AND task_id='${task}'` : ''}${personFilter ? ` AND user_id='${person.id}'` : ''}${custom ? ` AND spent_date='${day}'` : ''}`));

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  const errors = [];
  const requests = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (/\/api\/report_(time|detailed)/.test(request.url())) requests.push(request); });
  const projectPath = `/projects/${project}`;
  async function follow(name, taskFilter, personFilter, custom) {
    const link = page.getByRole('link', { name: new RegExp(`^${name}:`) });
    const href = await link.getAttribute('href');
    const query = new URL(href, base).searchParams;
    assert.equal(query.get('project_id'), project);
    assert.equal(query.get('task_id'), taskFilter ? task : null);
    assert.equal(query.get('user_id'), personFilter ? person.id : null);
    assert.equal(query.get('period'), custom ? 'custom' : 'all');
    assert.equal(query.get('from'), custom ? day : null);
    assert.equal(query.get('to'), custom ? day : null);
    await link.click();
    await expect(page.getByRole('combobox', { name: 'Project', exact: true })).toHaveValue(project);
    await expect(page.getByRole('combobox', { name: 'Task', exact: true })).toHaveValue(taskFilter ? task : '');
    await expect(page.getByRole('combobox', { name: 'Teammate', exact: true })).toHaveValue(personFilter ? person.id : '');
    await expect(page.getByRole('combobox', { name: 'Period', exact: true })).toHaveValue(custom ? 'custom' : 'all');
    await expect(page.locator('table tbody tr')).toHaveCount(count(taskFilter, personFilter, custom));
    const csv = await page.getByRole('link', { name: 'Export CSV', exact: true }).getAttribute('href');
    const result = await context.request.get(`${base}${csv}`);
    assert.equal(result.status(), 200);
    assert.equal((await result.text()).trim().split('\n').length - 1, count(taskFilter, personFilter, custom));
    await page.reload();
    await expect(page.locator('table tbody tr')).toHaveCount(count(taskFilter, personFilter, custom));
    await page.goBack();
    await expect(page.getByRole('region', { name: 'Project breakdown', exact: true })).toBeVisible();
  }
  try {
    sql(`BEGIN;
      INSERT INTO users (id,org_id,name,email,org_role)
        SELECT '${sortingPerson}',org_id,'Breakdown contributor','breakdown-fixture@example.com','member'
        FROM projects WHERE id='${project}';
      INSERT INTO projects (id,org_id,client_id,name,currency)
        SELECT '${sortingProject}',org_id,client_id,'Breakdown sorting fixture',currency FROM projects WHERE id='${project}';
      INSERT INTO project_tasks (project_id,task_id,billable)
        SELECT p.id,t.id,true FROM projects p JOIN tasks t ON t.org_id=p.org_id
        WHERE p.id='${sortingProject}' AND t.name IN ('Development','Design','Meetings');
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable)
        SELECT fixture.id::uuid,p.org_id,fixture.person::uuid,p.id,t.id,CURRENT_DATE,fixture.minutes,true
        FROM projects p JOIN tasks t ON t.org_id=p.org_id
        JOIN (VALUES
          ('01970000-0000-7000-8000-000000000702','Development','${admin}',180),
          ('01970000-0000-7000-8000-000000000703','Design','${sortingPerson}',90),
          ('01970000-0000-7000-8000-000000000704','Meetings','${admin}',90)
        ) fixture(id,task,person,minutes) ON fixture.task=t.name WHERE p.id='${sortingProject}';
      COMMIT;`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/projects/${sortingProject}`);
    const totalMinutes = Number(sql(`SELECT coalesce(sum(minutes),0) FROM time_entries WHERE project_id='${sortingProject}'`));
    assert.equal(totalMinutes, 360, 'Sorting fixture must contain all three task contributions');
    for (const label of ['Tasks', 'Team']) {
      const tab = page.getByRole('tab', { name: new RegExp(`^${label}`) });
      await tab.click();
      const table = page.getByRole('table', { name: new RegExp(`^${label} —`) });
      const rows = async () => table.locator('tbody tr').evaluateAll(nodes => nodes.map(row => ({
        name: row.querySelector('th').textContent.trim(),
        minutes: Number.parseInt(row.querySelector('td').title, 10),
      })));
      await expect(table).toBeVisible();
      const descending = await rows();
      assert.equal(descending.length, label === 'Tasks' ? 3 : 2, `${label} must contain every fixture entity`);
      assert.deepEqual(descending.map(row => row.minutes), descending.map(row => row.minutes).sort((a, b) => b - a));
      assert.equal(descending.reduce((sum, row) => sum + row.minutes, 0), totalMinutes);
      await expect(tab.locator('.chip')).toHaveText(String(descending.length));
      await expect(table.locator('tfoot td[title]')).toHaveAttribute('title', `${totalMinutes} minutes`);
      const ascendingButton = table.getByRole('button', { name: 'Sort hours ascending', exact: true });
      await ascendingButton.focus();
      await page.keyboard.press('Enter');
      await expect(table.locator('th[aria-sort]')).toHaveAttribute('aria-sort', 'ascending');
      assert.deepEqual(await rows(), [...descending].sort((a, b) => a.minutes - b.minutes), 'Equal-hour rows must retain their deterministic order');
      await table.getByRole('button', { name: 'Sort hours descending', exact: true }).click();
      await expect(table.locator('th[aria-sort]')).toHaveAttribute('aria-sort', 'descending');
      assert.deepEqual(await rows(), descending);
      const disclosure = table.locator('tbody button[aria-expanded]').first();
      await disclosure.focus();
      await page.keyboard.press('Enter');
      await expect(disclosure).toHaveAttribute('aria-expanded', 'true');
      assert.ok(await table.locator('tbody tr').count() > descending.length);
      const split = await disclosure.evaluate(button => {
        const parent = button.closest('tr');
        const minutes = row => Number.parseInt(row.querySelector('td').title, 10);
        let children = 0;
        for (let row = parent.nextElementSibling; row?.classList.contains('bg-tertiary'); row = row.nextElementSibling)
          children += minutes(row);
        return { parent: minutes(parent), children };
      });
      assert.equal(split.children, split.parent, 'Reciprocal child hours must reconcile to the parent');
      await page.keyboard.press('Enter');
      await expect(disclosure).toHaveAttribute('aria-expanded', 'false');
      assert.deepEqual(await rows(), descending, 'Closing the disclosure must restore the same parent rows');
    }
    console.log('Project breakdown: task/team counts, stable hour sorting, SQL totals and keyboard reciprocal disclosures passed');
    await page.goto(`${base}${projectPath}`);
    await follow('View Development time report', true, false, false);
    await expect(page.getByRole('tab', { name: /^Tasks/ })).toHaveAttribute('aria-selected', 'true');
    await expect(page.getByRole('link', { name: /time report: 0h$/ })).toHaveCount(0);
    await page.getByRole('tab', { name: /^Team/ }).click();
    await follow(`View ${person.name} time report`, false, true, false);
    await expect(page.getByRole('tab', { name: /^Team/ })).toHaveAttribute('aria-selected', 'true');
    await page.getByRole('table', { name: /^Team —/ }).getByRole('button', { name: new RegExp(person.name) }).click();
    await follow(`View ${person.name} / Development time report`, true, true, false);
    await expect(page.getByRole('tab', { name: /^Team/ })).toHaveAttribute('aria-selected', 'true');

    await page.locator('#project-report-period-trigger').click();
    await page.getByRole('menuitem', { name: 'Custom…', exact: true }).click();
    await page.locator('#project-report-from').fill(day);
    await page.locator('#project-report-to').fill(day);
    await page.getByRole('button', { name: 'Apply period', exact: true }).click();
    await expect.poll(() => new URL(page.url()).searchParams.get('from')).toBe(day);
    await follow(`View ${person.name} time report`, false, true, true);
    assert.equal(new URL(page.url()).searchParams.get('from'), day);
    assert.equal(new URL(page.url()).searchParams.get('to'), day);
    await expect(page.getByRole('tab', { name: /^Team/ })).toHaveAttribute('aria-selected', 'true');
    await follow('View total project time report', false, false, true);
    await page.getByRole('tab', { name: /^Tasks/ }).click();
    await follow('View Development time report', true, false, true);
    await page.getByRole('table', { name: /^Tasks —/ }).getByRole('button', { name: /Development/ }).click();
    await follow(`View Development / ${person.name} time report`, true, true, true);

    for (const query of ['project_id=invalid', 'task_id=', 'user_id=invalid', 'from=&to=&period=custom', 'from=2026-09-30&to=2026-09-01&period=custom', 'period=invalid']) {
      const before = requests.length;
      await page.goto(`${base}/reports?${query}`);
      await expect(page.getByRole('alert')).toBeVisible();
      await expect(page.locator('table')).toHaveCount(0);
      assert.equal(requests.length, before, `Invalid context fetched data: ${query}`);
    }
    sql(`UPDATE users SET org_role='member' WHERE id='${admin}'`);
    await page.goto(`${base}${projectPath}`);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.getByRole('link', { name: /time report:/ })).toHaveCount(0);
    await page.goto(`${base}/reports?project_id=${project}&period=all`);
    await expect(page.getByText('Manager or admin access is required to view reports.')).toBeVisible();
    await expect(page.locator('table')).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log('Project report links: task/person/reciprocal/total scope, periods, downloads, reload/back navigation, invalid context and role gates passed');
  } finally {
    try {
      sql(`BEGIN; UPDATE users SET org_role='admin' WHERE id='${admin}';
        DELETE FROM time_entries WHERE project_id='${sortingProject}';
        DELETE FROM project_tasks WHERE project_id='${sortingProject}';
        DELETE FROM projects WHERE id='${sortingProject}';
        DELETE FROM users WHERE id='${sortingPerson}'; COMMIT;`);
    } finally { await browser.close(); }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
