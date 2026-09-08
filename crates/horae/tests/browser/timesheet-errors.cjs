// Run only against an isolated, seeded, dev-login instance. This resets its own
// fixtures and exercises real deletion. Requires psql and Playwright on PATH.
// HORAE_TEST_URL=http://127.0.0.1:8092 HORAE_TEST_DATABASE_URL=postgres://... \
//   node crates/horae/tests/browser/timesheet-errors.cjs
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const database = process.env.HORAE_TEST_DATABASE_URL;
assert.ok(base && database, 'Set HORAE_TEST_URL and HORAE_TEST_DATABASE_URL to an isolated test instance');
const org = '01950000-0000-7000-8000-000000000001';
const user = '01950000-0000-7000-8000-000000000002';
const id = n => `019f2800-0000-7000-8000-${String(n).padStart(12, '0')}`;
const projectName = 'Timesheet error browser';
function sql(query) {
  return execFileSync('psql', ['-X', database, '-At', '-v', 'ON_ERROR_STOP=1', '-c', query],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}
function fixture() {
  assert.equal(sql(`SELECT week_start FROM organizations WHERE id = '${org}'`), '1');
  sql(`BEGIN;
    INSERT INTO clients (id, org_id, name, currency)
      VALUES ('${id(1)}', '${org}', '${projectName}', 'EUR') ON CONFLICT (id) DO NOTHING;
    INSERT INTO projects (id, org_id, client_id, name, currency)
      VALUES ('${id(2)}', '${org}', '${id(1)}', '${projectName}', 'EUR') ON CONFLICT (id) DO NOTHING;
    INSERT INTO tasks (id, org_id, name)
      VALUES ('${id(3)}', '${org}', '${projectName}') ON CONFLICT (id) DO NOTHING;
    INSERT INTO project_tasks (project_id, task_id, billable)
      VALUES ('${id(2)}', '${id(3)}', true) ON CONFLICT DO NOTHING;
    COMMIT;`);
  assert.equal(sql(`SELECT name FROM projects WHERE id = '${id(2)}' AND org_id = '${org}'`), projectName);
  sql(`INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, state, start_minute)
    VALUES
      ('${id(4)}', '${org}', '${user}', '${id(2)}', '${id(3)}', '2027-10-04', 60, true, 'open', 540),
      ('${id(5)}', '${org}', '${user}', '${id(2)}', '${id(3)}', '2027-10-05', 120, true, 'submitted', NULL),
      ('${id(6)}', '${org}', '${user}', '${id(2)}', '${id(3)}', '2027-10-06', 180, true, 'open', NULL)
    ON CONFLICT (id) DO UPDATE SET spent_date = EXCLUDED.spent_date,
      minutes = EXCLUDED.minutes, state = EXCLUDED.state, start_minute = EXCLUDED.start_minute
    WHERE time_entries.project_id = '${id(2)}';`);
}

(async () => {
  fixture();
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const pageErrors = [];
  const failures = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  async function open(mode) {
    const ready = page.waitForResponse(r => r.url().includes('/api/list_time_entries') && r.status() === 200);
    await page.goto(`${base}/timesheet/${mode}/2027-10-04?span=week`);
    await (await ready).finished();
  }
  async function check(name, run) {
    try { await run(); console.log(`PASS: ${name}`); }
    catch (error) { failures.push(`${name}: ${error.message}`); }
  }
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await check('partial row deletion reports counts, retains failed entry and supports retry', async () => {
      await open('week');
      const row = page.locator('.ts-body').filter({ hasText: projectName });
      const refreshed = page.waitForResponse(r => r.url().includes('/api/list_time_entries') && r.status() === 200);
      let release;
      let started;
      const gate = new Promise(resolve => { release = resolve; });
      const pending = new Promise(resolve => { started = resolve; });
      let attempts = 0;
      const pattern = '**/api/delete_time_entry*';
      await page.route(pattern, async route => {
        attempts += 1;
        if (attempts === 1) { started(); await gate; }
        await route.continue();
      });
      try {
        await row.getByRole('button', { name: 'Remove row', exact: true }).click();
        await pending;
        await expect(row.getByRole('button', { name: 'Remove row', exact: true })).toBeDisabled();
      } finally {
        release();
        await (await refreshed).finished();
        await page.unroute(pattern);
      }
      assert.equal(attempts, 3);
      assert.equal(sql(`SELECT string_agg(id::text, ',') FROM time_entries WHERE project_id = '${id(2)}'`), id(5));
      await expect(page.getByRole('alert')).toContainText('Deleted 2 of 3 entries');
      await expect(row.locator('.ts-rowtotal')).toHaveText('2:00');
      const retried = page.waitForResponse(r => r.url().includes('/api/list_time_entries') && r.status() === 200);
      await row.getByRole('button', { name: 'Remove row', exact: true }).click();
      await (await retried).finished();
      await expect(page.getByRole('alert')).toContainText('Deleted 0 of 1 entries');
      sql(`UPDATE time_entries SET state = 'open' WHERE id = '${id(5)}' AND project_id = '${id(2)}'`);
      await row.getByRole('button', { name: 'Remove row', exact: true }).click();
      await expect(row).toHaveCount(0);
      await expect(page.getByRole('alert')).toHaveCount(0);
    });

    fixture();
    for (const kind of ['move', 'resize', 'reorder', 'lost response']) {
      await check(`${kind} failure is visible and reconciles with saved data`, async () => {
        await open('calendar');
        const endpoint = kind === 'reorder' ? 'reorder_untimed_entries' : 'reschedule_time_entry';
        const pattern = `**/api/${endpoint}*`;
        let committedStatus;
        await page.route(pattern, async route => {
          // A transport failure does not prove that the server rejected a write.
          if (kind === 'lost response') committedStatus = (await route.fetch()).status();
          await route.abort('failed');
        });
        try {
          const column = kind === 'reorder' ? 2 : 0;
          const event = page.locator('.ts-cal-col').nth(column).locator('.ts-cal-event').filter({ hasText: projectName });
          await event.scrollIntoViewIfNeeded();
          const source = kind === 'resize' ? event.locator('.ts-cal-resize') : event;
          const box = await source.boundingBox();
          assert.ok(box);
          const x = box.x + box.width / 2;
          const y = box.y + (kind === 'resize' ? box.height / 2 : 12);
          const rejected = page.waitForEvent('requestfailed', r => r.url().includes(`/api/${endpoint}`));
          const refreshed = page.waitForResponse(r => r.url().includes('/api/list_time_entries') && r.status() === 200);
          await page.mouse.move(x, y);
          await page.mouse.down();
          if (kind === 'reorder') {
            const target = await page.locator('.ts-cal-col').nth(3).boundingBox();
            await page.mouse.move(target.x + target.width / 2, y + 30, { steps: 5 });
          } else {
            await page.mouse.move(x, y + 65, { steps: 5 });
          }
          await page.mouse.up();
          await rejected;
          await expect(page.getByRole('alert')).toContainText('Could not change entry');
          await (await refreshed).finished();
          if (kind === 'lost response') {
            assert.equal(committedStatus, 200);
            const start = Number(sql(`SELECT start_minute FROM time_entries WHERE id = '${id(4)}'`));
            assert.notEqual(start, 540);
            const clock = minute => `${Math.floor(minute / 60)}:${String(minute % 60).padStart(2, '0')}`;
            await expect(event.locator('.ts-cal-ev-time')).toHaveText(`${clock(start)}–${clock(start + 60)}`);
          } else {
            assert.equal(sql(`SELECT spent_date || '/' || minutes || '/' || COALESCE(start_minute::text, 'none') FROM time_entries WHERE id = '${id(kind === 'reorder' ? 6 : 4)}'`),
              kind === 'reorder' ? '2027-10-06/180/none' : '2027-10-04/60/540');
          }
        } finally { await page.unroute(pattern); }
      });
    }
    await check('day-view timer failure is visible', async () => {
      await open('day');
      const pattern = '**/api/start_timer*';
      await page.route(pattern, route => route.abort('failed'));
      try {
        const rejected = page.waitForEvent('requestfailed', r => r.url().includes('/api/start_timer'));
        const refreshed = page.waitForResponse(r => r.url().includes('/api/list_time_entries') && r.status() === 200);
        await page.locator('.ts-day-entry').filter({ hasText: projectName }).getByRole('button', { name: 'Start', exact: true }).click();
        await rejected;
        await expect(page.getByRole('alert')).toContainText('Could not start timer');
        await (await refreshed).finished();
      } finally { await page.unroute(pattern); }
    });
    assert.deepEqual(pageErrors, []);
    assert.deepEqual(failures, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
