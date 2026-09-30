// Reads and temporarily changes roles only in the runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.ok(database.pathname === '/horae' && database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const task = sql("SELECT id FROM tasks WHERE name='Development'");
const project = sql("SELECT id FROM projects WHERE code='ACME-01'");
const admin = sql("SELECT id FROM users WHERE email='admin@example.com'");
const from = sql('SELECT min(spent_date) FROM time_entries');
const to = sql('SELECT max(spent_date) FROM time_entries');
const expected = Number(sql(`SELECT sum(minutes) FROM time_entries WHERE project_id='${project}' AND task_id='${task}'`));
const expectedCount = Number(sql(`SELECT count(*) FROM time_entries WHERE project_id='${project}' AND task_id='${task}'`));

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.route('**/api/list_tasks*', route => route.abort(), { times: 1 });
    await page.goto(`${base}/reports`);
    await expect(page.getByRole('alert')).toContainText('Could not load report tasks');
    await page.getByRole('button', { name: 'Retry tasks', exact: true }).click();
    const tasks = page.getByRole('combobox', { name: 'Task', exact: true });
    await expect(tasks.locator(`option[value="${task}"]`)).toHaveCount(1);
    await page.locator('input[type="date"]').first().fill(from);
    await page.locator('input[type="date"]').last().fill(to);
    await page.getByRole('combobox', { name: 'Project', exact: true }).selectOption(project);
    const matches = response => {
      const data = response.request().postDataJSON();
      return data?.task_id === task && data?.project_id === project && data?.from === from && data?.to === to;
    };
    const summaryRead = page.waitForResponse(r => r.url().includes('/api/report_time') && matches(r));
    const detailRead = page.waitForResponse(r => r.url().includes('/api/report_detailed') && matches(r));
    await tasks.selectOption(task);
    const summaryResponse = await summaryRead;
    const summary = await summaryResponse.json();
    assert.equal(summaryResponse.status(), 200);
    assert.equal(summary.reduce((sum, row) => sum + row.total_minutes, 0), expected);
    const detailResponse = await detailRead;
    const details = await detailResponse.json();
    assert.equal(details.length, expectedCount);
    assert.ok(details.every(row => row.task_name === 'Development'));
    assert.equal(details.reduce((sum, row) => sum + row.minutes, 0), expected);
    await page.getByRole('button', { name: 'Detailed time', exact: true }).click();
    await expect(page.locator('table tbody tr')).toHaveCount(expectedCount);
    for (const width of [320, 768, 1440]) {
      await page.setViewportSize({ width, height: 700 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
      const bounds = await tasks.boundingBox();
      assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width + 1);
    }
    for (const format of ['CSV', 'XLSX']) {
      const link = page.getByRole('link', { name: `Export ${format}`, exact: true });
      const href = await link.getAttribute('href');
      const query = new URL(href, base).searchParams;
      assert.equal(query.get('task_id'), task);
      assert.equal(query.get('project_id'), project);
      assert.equal(query.get('from'), from);
      assert.equal(query.get('to'), to);
      const response = await context.request.get(`${base}${href}`);
      assert.equal(response.status(), 200);
      if (format === 'CSV') {
        const rows = (await response.text()).trim().split('\n').slice(1);
        assert.equal(rows.length, expectedCount);
        assert.ok(rows.every(row => row.split(',')[2] === 'Development'));
      } else {
        assert.match(response.headers()['content-type'], /spreadsheetml/);
        assert.equal((await response.body()).subarray(0, 2).toString(), 'PK');
      }
    }
    for (const response of [summaryResponse, detailResponse]) {
      const invalid = { ...response.request().postDataJSON(), task_id: 'invalid-task' };
      assert.equal((await context.request.post(response.url(), { data: invalid })).status(), 400);
    }
    sql(`UPDATE users SET org_role='member' WHERE id='${admin}'`);
    for (const response of [summaryResponse, detailResponse]) {
      const denied = await context.request.post(response.url(), { data: response.request().postDataJSON() });
      assert.equal(denied.status(), 403);
    }
    for (const format of ['CSV', 'XLSX']) {
      const href = await page.getByRole('link', { name: `Export ${format}`, exact: true }).getAttribute('href');
      assert.equal((await context.request.get(`${base}${href}`)).status(), 403);
    }
    assert.deepEqual(errors, []);
    console.log('Report task filter: catalog retry, project/task/date intersection, summary/detail/CSV/XLSX parity and revoked role passed');
  } finally {
    sql(`UPDATE users SET org_role='admin' WHERE id='${admin}'`);
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
