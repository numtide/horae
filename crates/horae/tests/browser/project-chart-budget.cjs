// Budget fixtures and role changes belong only to the disposable browser runner.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.ok(database.pathname === '/horae' && database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const project = '01970000-0000-7000-8000-000000000401';
const admin = sql("SELECT id FROM users WHERE email='admin@example.com'");
const today = new Date().toISOString().slice(0, 10);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  let summaryReads = 0;
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (request.url().includes('/api/get_project_summary')) summaryReads++; });
  try {
    sql(`BEGIN;
      INSERT INTO projects (id,org_id,client_id,name,currency,budget_kind,budget_minutes)
        SELECT '${project}',c.org_id,c.id,'Chart budget fixture',c.currency,'hours',240
        FROM clients c JOIN organizations o ON o.id=c.org_id WHERE o.name='Demo Org' ORDER BY c.id LIMIT 1;
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable)
        SELECT '01970000-0000-7000-8000-000000000402',p.org_id,'${admin}',p.id,t.id,'${today}',90,true
        FROM projects p JOIN tasks t ON t.org_id=p.org_id AND t.name='Development' WHERE p.id='${project}';
      COMMIT;`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/projects/${project}`);
    const activity = page.getByRole('region', { name: 'Project activity', exact: true });
    const reference = activity.locator('.project-activity-budget');
    const summary = page.getByRole('region', { name: 'Project summary', exact: true });
    await expect(activity).toContainText('Project hours budget: 4h (240 minutes)');
    await expect(reference).toHaveAttribute('d', 'M0 0 H10000');
    await expect(activity.getByRole('img')).toHaveAttribute('aria-label', /Selected period total: 1.5h/);
    await expect(summary).toContainText('Budget: 4h');
    assert.equal(summaryReads, 1, 'Chart and tiles must share the summary request');
    await activity.getByRole('button', { name: 'Hours per week', exact: true }).click();
    await expect(reference).toHaveCount(0);
    await activity.getByRole('button', { name: 'Project progress', exact: true }).click();
    await expect(reference).toHaveCount(1);
    const nav = activity.getByRole('group', { name: 'Chart week navigation', exact: true });
    assert.deepEqual(await nav.locator('.ts-pager > button').evaluateAll(buttons => buttons.map(button => button.getAttribute('aria-label'))),
      ['Previous week', await page.locator('#project-chart-week').getAttribute('aria-label'), 'Next week']);
    await nav.getByRole('button', { name: 'Previous week', exact: true }).focus();
    await page.keyboard.press('Tab');
    const weekPicker = page.locator('#project-chart-week');
    await expect(weekPicker).toBeFocused();
    await expect.poll(() => weekPicker.evaluate(node => getComputedStyle(node).boxShadow)).toContain('inset');
    await page.addStyleTag({ content: '#__dx-toast { display: none !important; }' });
    for (const [width, size] of [[320, 16], [390, 16], [768, 16], [1440, 16], [320, 32], [1440, 32]]) {
      await page.setViewportSize({ width, height: 600 });
      await page.evaluate(font => { document.documentElement.style.fontSize = `${font}px`; }, size);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), `Page overflow at ${width}/${size}`);
      const controls = nav.getByRole('button');
      await expect(controls).toHaveCount(4);
      for (const button of await controls.all()) {
        const bounds = await button.boundingBox();
        assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width + 1 && bounds.height >= 44);
        assert.ok(await button.evaluate(node => node.scrollWidth <= node.clientWidth + 1), 'Grouped week controls must not clip enlarged labels');
      }
    }
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    await page.setViewportSize({ width: 1440, height: 900 });
    assert.equal(summaryReads, 1);
    await page.locator('#project-report-period-trigger').click();
    await page.getByRole('menuitem', { name: 'This month', exact: true }).click();
    await expect(reference).toHaveCount(0);
    assert.equal(summaryReads, 1, 'Changing report scope must not reload the lifetime summary');

    sql(`INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,monthly_reset,include_nonbillable)
      SELECT '01970000-0000-7000-8000-000000000403',org_id,id,'${admin}','project',true,true FROM projects WHERE id='${project}'`);
    await page.reload();
    await expect(reference).toHaveCount(1);
    await expect(activity).toContainText('Current allowance, not rounded consumption');
    await page.locator('#project-report-period-trigger').click();
    await page.getByRole('menuitem', { name: 'All time', exact: true }).click();
    await expect(reference).toHaveCount(0);
    await page.locator('#project-report-period-trigger').click();
    await page.getByRole('menuitem', { name: 'This month', exact: true }).click();
    await expect(reference).toHaveCount(1);

    let rejectSummary;
    const gate = new Promise(resolve => { rejectSummary = resolve; });
    await page.route('**/api/get_project_summary*', async route => { await gate; await route.abort(); }, { times: 1 });
    await page.reload();
    await expect(activity.getByRole('img')).toBeVisible();
    await expect(summary.getByRole('status').filter({ hasText: 'Loading project summary' })).toBeVisible();
    await expect(reference).toHaveCount(0);
    rejectSummary();
    await expect(summary.getByRole('alert')).toContainText('Could not load project summary');
    await expect(reference).toHaveCount(0);
    const beforeRetry = summaryReads;
    await summary.getByRole('button', { name: 'Retry summary', exact: true }).click();
    await expect(reference).toHaveCount(1);
    assert.equal(summaryReads, beforeRetry + 1);

    sql(`UPDATE projects SET budget_kind='amount',budget_amount_cents=24000 WHERE id='${project}'`);
    await page.reload();
    await expect(summary).toContainText('Budget:');
    await expect(activity.getByRole('img')).toBeVisible();
    await expect(reference).toHaveCount(0);
    sql(`UPDATE users SET org_role='member' WHERE id='${admin}'`);
    await page.reload();
    await expect(summary.getByRole('alert')).toBeVisible();
    await expect(reference).toHaveCount(0);
    assert.deepEqual(errors, []);
    console.log('Project chart budgets: shared summary, exact hours, grouped controls/reflow, cumulative-only scale, matching monthly/lifetime periods, loading/error/retry, money and permission gates passed');
  } finally {
    try {
      sql(`BEGIN; UPDATE users SET org_role='admin' WHERE id='${admin}';
        DELETE FROM time_entries WHERE project_id='${project}';
        DELETE FROM projects WHERE id='${project}'; COMMIT;`);
    } finally { await browser.close(); }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
