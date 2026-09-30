// Run against run-design-checks.sh's disposable database, never imported data.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const project = '01970000-0000-7000-8000-000000000301';
const today = new Date().toISOString().slice(0, 10);
const shift = (date, days) => {
  const result = new Date(`${date}T00:00:00Z`);
  result.setUTCDate(result.getUTCDate() + days);
  return result.toISOString().slice(0, 10);
};
const sunday = shift(today, -new Date(`${today}T00:00:00Z`).getUTCDay());
const old = shift(sunday, -30 * 7);
const previousWeekEnd = shift(sunday, -1);
const dateLabel = date => new Date(`${date}T00:00:00Z`).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });
const originalWeekStart = Number(sql("SELECT week_start FROM organizations WHERE name='Demo Org'"));
assert.ok(Number.isInteger(originalWeekStart) && originalWeekStart >= 1 && originalWeekStart <= 7);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const reads = { activity: 0, breakdown: 0, invoices: 0 };
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    for (const name of Object.keys(reads)) {
      if (request.url().includes(`/api/get_project_${name}`)) reads[name]++;
    }
  });
  try {
    sql(`BEGIN;
      UPDATE organizations SET week_start=7 WHERE name='Demo Org';
      INSERT INTO projects (id,org_id,client_id,name,code,currency)
        SELECT '${project}',c.org_id,c.id,'Chart navigation fixture','CHART-WEEKS',c.currency
        FROM clients c JOIN organizations o ON o.id=c.org_id WHERE o.name='Demo Org'
        ORDER BY c.id LIMIT 1;
      INSERT INTO project_tasks (project_id,task_id,billable)
        SELECT '${project}',t.id,true FROM tasks t JOIN projects p ON p.org_id=t.org_id
        WHERE p.id='${project}' AND t.name='Development';
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable)
        SELECT fixture.id::uuid,p.org_id,u.id,p.id,t.id,fixture.day::date,fixture.minutes,true
        FROM projects p JOIN users u ON u.org_id=p.org_id AND u.email='admin@example.com'
        JOIN tasks t ON t.org_id=p.org_id AND t.name='Development'
        CROSS JOIN (VALUES
          ('01970000-0000-7000-8000-000000000302','${old}',60),
          ('01970000-0000-7000-8000-000000000303','${today}',120)
        ) fixture(id,day,minutes) WHERE p.id='${project}';
      COMMIT;`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    const response = page.waitForResponse(r => r.url().includes('/api/get_project_activity') && r.status() === 200);
    await page.goto(`${base}/projects/${project}`);
    const data = await (await response).json();
    assert.equal(data.week_start, 'Sun');
    assert.equal(data.weeks.length, 31);
    assert.equal(data.weeks.at(-1).cumulative_minutes, 180);
    const activity = page.getByRole('region', { name: 'Project activity', exact: true });
    const previous = activity.getByRole('button', { name: 'Previous week', exact: true });
    const next = activity.getByRole('button', { name: 'Next week', exact: true });
    const current = activity.getByRole('button', { name: 'This week', exact: true });
    const chart = activity.getByRole('img');
    await expect(chart).toHaveAttribute('aria-label', new RegExp(`${shift(sunday, -25 * 7)} to ${today}`));
    await expect(activity.locator('.project-activity-line')).toHaveAttribute('d', /^M0 1334 /);
    await expect(next).toBeDisabled();
    await expect(current).toBeDisabled();
    const picker = activity.getByRole('button', { name: /^Choose chart week/ });
    const calendar = page.getByRole('dialog', { name: 'Choose chart week', exact: true });
    await picker.focus();
    await page.keyboard.press('Enter');
    await expect(calendar).toBeVisible();
    await expect.poll(() => calendar.evaluate(node => node.contains(document.activeElement))).toBe(true);
    await expect(calendar.locator('.grid-cols-7').first().locator('div').first()).toHaveText('Su');
    await expect(calendar.getByRole('button', { name: dateLabel(shift(sunday, 7)), exact: true })).toBeDisabled();
    await expect(calendar.getByRole('button', { name: dateLabel(shift(sunday, 6)), exact: true })).toBeEnabled();
    const lastAllowed = shift(sunday, 6);
    if (sunday.slice(0, 7) !== lastAllowed.slice(0, 7)) {
      await calendar.getByRole('button', { name: 'Next month', exact: true }).click();
    }
    await expect(calendar.getByRole('button', { name: 'Next month', exact: true })).toBeDisabled();
    await calendar.getByRole('button', { name: 'Previous month', exact: true }).click();
    await page.keyboard.press('Escape');
    await expect(calendar).toBeHidden();
    await expect(picker).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(calendar.getByRole('button', { name: dateLabel(sunday), exact: true })).toHaveClass(/picked/);
    const priorSunday = calendar.getByRole('button', { name: dateLabel(shift(sunday, -7)), exact: true });
    if (!(await priorSunday.count())) await calendar.getByRole('button', { name: 'Previous month', exact: true }).click();
    await priorSunday.focus();
    await page.keyboard.press('Enter');
    await expect(calendar).toBeHidden();
    await expect(picker).toBeFocused();
    await expect(chart).toHaveAttribute('aria-label', new RegExp(`to ${previousWeekEnd}`));
    await current.click();
    await expect(chart).toHaveAttribute('aria-label', new RegExp(`to ${today}`));
    await expect(page.getByRole('tab', { name: /^Tasks/ })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Invoiced', exact: true })).toContainText('No invoices');
    const initialReads = { ...reads };
    assert.deepEqual(initialReads, { activity: 1, breakdown: 1, invoices: 1 });
    await previous.focus();
    await page.keyboard.press('Enter');
    await expect(chart).toHaveAttribute('aria-label', new RegExp(`to ${previousWeekEnd}`));
    await expect(chart).toHaveAttribute('aria-label', /Selected period total: 3h/);
    await expect(activity.locator('.project-activity-line')).toHaveAttribute('d', /^M0 0 /);
    await expect(next).toBeEnabled();
    await expect(current).toBeEnabled();
    await activity.getByRole('button', { name: 'Hours per week', exact: true }).click();
    await expect(chart).toBeVisible();
    await expect(activity.locator('.project-activity-bars')).toHaveCount(1);
    assert.ok([...((await activity.locator('.project-activity-bars').getAttribute('d')).matchAll(/v-(\d+)/g))].every(match => Number(match[1]) === 0));
    await expect(activity).toContainText('No time tracked in this chart window');
    await activity.locator('summary').filter({ hasText: 'View weekly data' }).click();
    await expect(activity.locator('tbody tr')).toHaveCount(31);
    await expect(activity.locator('tbody tr').first()).toContainText(old);
    await expect(activity.locator('tbody tr').last()).toContainText('180');
    await next.click();
    await expect(next).toBeDisabled();
    await expect(activity.locator('.project-activity-bars')).toBeVisible();
    await previous.click();
    await current.click();
    await expect(chart).toHaveAttribute('aria-label', new RegExp(`to ${today}`));
    assert.deepEqual(reads, initialReads, 'Chart navigation must not refetch or change the report');
    for (const width of [320, 390, 768, 1440]) {
      await page.setViewportSize({ width, height: 600 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
      for (const control of [previous, next, current]) {
        assert.ok((await control.boundingBox()).height >= 44);
      }
      await picker.click();
      await expect(calendar).toBeVisible();
      const bounds = await calendar.boundingBox();
      assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width + 1 && bounds.y >= 0 && bounds.y + bounds.height <= 601);
      assert.equal(await calendar.evaluate(node => node.scrollWidth <= node.clientWidth + 1), true);
      await page.keyboard.press('Escape');
      await expect(picker).toBeFocused();
    }
    // Text enlargement is separate from the narrow viewport/reflow checks.
    const enlargedText = await page.addStyleTag({ content: 'html { font-size: 200% !important; }' });
    for (const width of [1440, 320]) {
      await page.setViewportSize({ width, height: 600 });
      await picker.click();
      await expect(calendar).toBeVisible();
      assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).fontSize), '32px');
      for (const element of await calendar.locator('.dp, .dp > div, .dp button').all()) {
        const size = await element.evaluate(node => ({ label: node.getAttribute('aria-label') || node.className, client: node.clientWidth, scroll: node.scrollWidth }));
        assert.ok(size.scroll <= size.client + 1, `Calendar must not clip enlarged content at ${width}px: ${JSON.stringify(size)}`);
      }
      await page.keyboard.press('Escape');
      await expect(picker).toBeFocused();
    }
    await enlargedText.evaluate(node => node.remove());
    await page.setViewportSize({ width: 1440, height: 600 });
    await picker.click();
    await activity.getByRole('button', { name: 'Hours per week', exact: true }).click();
    await expect(calendar).toBeHidden();
    await expect(picker).toHaveAttribute('aria-expanded', 'false');
    assert.deepEqual(reads, initialReads, 'Opening and dismissing the calendar must not refetch');
    const breakdown = page.getByRole('region', { name: 'Project breakdown', exact: true });
    const reporting = breakdown.getByRole('region', { name: 'Project reporting period', exact: true });
    await page.getByRole('tab', { name: /^Team/ }).click();
    await reporting.getByRole('button', { name: /^All time/ }).click();
    await page.getByRole('menuitem', { name: 'Custom…', exact: true }).click();
    await reporting.getByLabel('Start date', { exact: true }).fill(today);
    await reporting.getByLabel('End date', { exact: true }).fill(old);
    await reporting.getByRole('button', { name: 'Apply period', exact: true }).click();
    await expect(reporting.getByRole('alert')).toBeVisible();
    assert.deepEqual(reads, initialReads, 'Invalid dates must not change the report');
    await reporting.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(reporting.getByRole('heading')).toHaveText('All time');
    await expect(reporting.getByRole('button', { name: /^All time/ })).toBeFocused();
    await reporting.getByRole('button', { name: /^All time/ }).click();
    await page.getByRole('menuitem', { name: 'Custom…', exact: true }).click();
    await reporting.getByLabel('Start date', { exact: true }).fill(old);
    await reporting.getByLabel('End date', { exact: true }).fill(old);
    let rejectBreakdown;
    const breakdownGate = new Promise(resolve => { rejectBreakdown = resolve; });
    await page.route('**/api/get_project_breakdown*', async route => {
      await breakdownGate;
      await route.abort();
    }, { times: 1 });
    await reporting.getByRole('button', { name: 'Apply period', exact: true }).click();
    await expect(reporting.getByRole('button', { name: /^Custom period/ })).toBeFocused();
    await expect(chart).toHaveAttribute('aria-label', new RegExp(`${old} to ${old}`));
    await expect(chart).toHaveAttribute('aria-label', /Selected period total: 1h/);
    await expect(page.getByRole('tab', { name: /^Team/ })).toHaveAttribute('aria-selected', 'true');
    await expect(breakdown.getByRole('status')).toHaveText('Loading project breakdown…');
    await expect(breakdown.getByRole('table')).toHaveCount(0);
    await expect(reporting.getByRole('heading')).toHaveText('Custom period');
    await page.getByRole('tab', { name: /^Invoices/ }).click();
    await expect(reporting).toBeHidden();
    await expect(breakdown).toContainText('No invoices for this project');
    await page.getByRole('tab', { name: /^Team/ }).click();
    await expect(reporting.getByRole('heading')).toHaveText('Custom period');
    rejectBreakdown();
    await expect(breakdown.getByRole('alert')).toContainText('Could not load project breakdown');
    await expect(reporting.getByRole('button', { name: /^Custom period/ })).toBeEnabled();
    await breakdown.getByRole('button', { name: 'Retry breakdown', exact: true }).click();
    await expect(page.getByRole('tab', { name: /^Team/ })).toHaveAttribute('aria-selected', 'true');
    await expect(breakdown.getByRole('table')).toHaveAttribute('aria-label', /^Team/);
    await expect(breakdown.getByRole('table').locator('tfoot')).toContainText('1h');
    assert.equal(reads.breakdown, 3, 'Period change and retry each make one breakdown read');
    const largeReportText = await page.addStyleTag({ content: 'html { font-size: 200% !important; }' });
    for (const width of [320, 768, 1440]) {
      await page.setViewportSize({ width, height: 600 });
      for (const control of [reporting.getByRole('heading'), reporting.getByRole('button', { name: /^Custom period/ })]) {
        const bounds = await control.boundingBox();
        assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width + 1, `Report toolbar must reflow at ${width}px`);
        assert.equal(await control.evaluate(node => node.scrollWidth <= node.clientWidth + 1), true);
      }
    }
    await largeReportText.evaluate(node => node.remove());
    await page.setViewportSize({ width: 1440, height: 600 });
    await expect(activity.locator('tbody tr')).toHaveCount(1);
    await current.click();
    await expect(chart).toHaveCount(0);
    await expect(activity).toContainText('No selected-period dates in this chart window');
    await expect(activity.locator('tbody tr')).toHaveCount(1);
    assert.equal(reads.activity, 2);
    assert.equal(reads.invoices, 1);
    await page.getByRole('link', { name: 'Back to Projects', exact: true }).click();
    await page.route('**/api/get_project_activity*', route => route.abort(), { times: 1 });
    await page.locator(`a[href="/projects/${project}?"], a[href="/projects/${project}"]`).first().click();
    await expect(activity.getByRole('alert')).toContainText('Project activity is unavailable');
    await expect(chart).toHaveCount(0);
    await expect(previous).toHaveCount(0);
    await activity.getByRole('button', { name: 'Retry activity', exact: true }).click();
    await expect(chart).toHaveAttribute('aria-label', new RegExp(`${shift(sunday, -25 * 7)} to ${today}`));
    await expect(chart).toHaveAttribute('aria-label', /Selected period total: 3h/);
    await expect(current).toBeDisabled();
    await expect(next).toBeDisabled();
    assert.equal(reads.activity, 4, 'Re-entry and retry each make one read');
    assert.deepEqual(errors, []);
    console.log('Project activity: configured weeks, bounded calendar, keyboard/focus/dismissal, enlarged text, cumulative carry-in, no refetch, report toolbar validation, pending/error/retry tab preservation, invoice isolation and responsive bounds passed');
  } finally {
    try {
      sql(`BEGIN;
        DELETE FROM time_entries WHERE project_id='${project}';
        DELETE FROM project_tasks WHERE project_id='${project}';
        DELETE FROM projects WHERE id='${project}';
        UPDATE organizations SET week_start=${originalWeekStart} WHERE name='Demo Org';
        COMMIT;`);
    } finally {
      await browser.close();
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
