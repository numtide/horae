// Run with run-design-checks.sh timesheet-errors; only its disposable DB is allowed.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const database = process.env.DATABASE_URL;
const target = new URL(base);
const dbTarget = new URL(database);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(dbTarget.pathname, '/horae');
assert.match(dbTarget.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
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
      VALUES ('${id(1)}', '${org}', '${projectName}', 'EUR');
    INSERT INTO projects (id, org_id, client_id, name, currency)
      VALUES ('${id(2)}', '${org}', '${id(1)}', '${projectName}', 'EUR');
    INSERT INTO tasks (id, org_id, name)
      VALUES ('${id(3)}', '${org}', '${projectName}');
    INSERT INTO project_tasks (project_id, task_id, billable)
      VALUES ('${id(2)}', '${id(3)}', true);
    INSERT INTO assignments (id, project_id, user_id)
      VALUES ('${id(7)}', '${id(2)}', '${user}');
    COMMIT;`);
  assert.equal(sql(`SELECT name FROM projects WHERE id = '${id(2)}' AND org_id = '${org}'`), projectName);
}
function resetEntries() {
  sql(`INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, state, start_minute)
    VALUES
      ('${id(4)}', '${org}', '${user}', '${id(2)}', '${id(3)}', '2027-10-04', 60, true, 'open', 540),
      ('${id(5)}', '${org}', '${user}', '${id(2)}', '${id(3)}', '2027-10-05', 120, true, 'open', NULL),
      ('${id(6)}', '${org}', '${user}', '${id(2)}', '${id(3)}', '2027-10-06', 180, true, 'open', NULL)
    ON CONFLICT (id) DO UPDATE SET spent_date = EXCLUDED.spent_date,
      minutes = EXCLUDED.minutes, state = EXCLUDED.state, start_minute = EXCLUDED.start_minute
    WHERE time_entries.project_id = '${id(2)}';`);
}

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const pageErrors = [];
  const failures = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  async function open(mode) {
    const ready = page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
    await page.goto(`${base}/timesheet/${mode}/2027-10-04?span=week`);
    await (await ready).finished();
  }
  async function check(name, run) {
    try { await run(); console.log(`PASS: ${name}`); }
    catch (error) { failures.push(`${name}: ${error.message}`); }
  }
  let created = false;
  const pattern = '**/api/apply_timesheet_command*';
  function command(request) {
    const body = request.postDataJSON();
    assert.deepEqual(body.context, {
      expected_requester: { org_id: org, user_id: user },
      subject_id: user,
      expected_policy: 'legacy_own',
    });
    return body.command;
  }
  try {
    fixture();
    created = true;
    resetEntries();
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await check('row deletion is atomic after a stale-client lock and supports retry', async () => {
      await open('week');
      const row = page.locator('.ts-body').filter({ hasText: projectName });
      await expect(row.locator('.ts-rowtotal')).toHaveText('6:00');
      sql(`UPDATE time_entries SET state = 'submitted' WHERE id = '${id(5)}'`);
      let release;
      const gate = new Promise(resolve => { release = resolve; });
      let attempts = 0;
      let captured;
      await page.route(pattern, async route => {
        attempts += 1;
        captured = command(route.request());
        await gate;
        await route.continue();
      });
      try {
        await row.getByRole('button', { name: 'Remove row', exact: true }).click();
        await expect.poll(() => attempts).toBe(1);
        assert.equal(captured.operation, 'delete');
        assert.deepEqual(captured.entry_ids.slice().sort(), [id(4), id(5), id(6)]);
        await expect(row.getByRole('button', { name: 'Remove row', exact: true })).toBeDisabled();
        const refreshed = page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
        release();
        await (await refreshed).finished();
        await expect(page.getByRole('alert')).toContainText('Could not remove row');
      } finally {
        release();
        await page.unroute(pattern);
      }
      assert.equal(attempts, 1);
      assert.equal(sql(`SELECT string_agg(id::text, ',' ORDER BY id) FROM time_entries WHERE project_id = '${id(2)}'`), [id(4), id(5), id(6)].join(','));
      await expect(row.locator('.ts-rowtotal')).toHaveText('6:00');
      sql(`UPDATE time_entries SET state = 'open' WHERE id = '${id(5)}' AND project_id = '${id(2)}'`);
      await open('week');
      await row.getByRole('button', { name: 'Remove row', exact: true }).click();
      await expect(row).toHaveCount(0);
      await expect(page.getByRole('alert')).toHaveCount(0);
      assert.equal(sql(`SELECT count(*) FROM time_entries WHERE project_id = '${id(2)}'`), '0');
    });

    for (const kind of ['move', 'resize', 'reorder', 'lost response']) {
      await check(`${kind} failure is visible and reconciles with saved data`, async () => {
        resetEntries();
        await open('calendar');
        const operation = kind === 'reorder' ? 'reorder' : 'reschedule';
        let committedStatus;
        await page.route(pattern, async route => {
          assert.equal(command(route.request()).operation, operation);
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
          const rejected = page.waitForEvent('requestfailed', r => r.url().includes('/api/apply_timesheet_command'));
          const refreshed = page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
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
    // No current tracking choices must not disable legacy historical edits.
    // Hide unrelated seed choices without changing their projects or assignments.
    const trackingPattern = '**/api/load_timesheet_tracking*';
    await page.route(trackingPattern, async route => {
      const response = await route.fetch();
      assert.equal(response.status(), 200);
      await route.fulfill({ response, json: [] });
    });
    sql(`UPDATE projects SET active=false WHERE id='${id(2)}'`);
    try {
      for (const kind of ['move', 'resize', 'reorder']) {
        await check(`historical ${kind} works without current tracking choices`, async () => {
          resetEntries();
          await open('calendar');
          await expect(page.getByRole('button', { name: 'Add entry', exact: true })).toBeDisabled();
          const column = kind === 'reorder' ? 2 : 0;
          const event = page.locator('.ts-cal-col').nth(column).locator('.ts-cal-event').filter({ hasText: projectName });
          await expect(event).not.toHaveClass(/locked/);
          await event.scrollIntoViewIfNeeded();
          const source = kind === 'resize' ? event.locator('.ts-cal-resize') : event;
          const box = await source.boundingBox();
          assert.ok(box);
          const x = box.x + box.width / 2;
          const y = box.y + (kind === 'resize' ? box.height / 2 : 12);
          await page.mouse.move(x, y);
          await page.mouse.down();
          if (kind === 'reorder') {
            const target = await page.locator('.ts-cal-col').nth(3).boundingBox();
            await page.mouse.move(target.x + target.width / 2, y + 30, { steps: 5 });
          } else {
            await page.mouse.move(x, y + 65, { steps: 5 });
          }
          await page.mouse.up();
          if (kind === 'reorder') {
            await expect.poll(() => sql(`SELECT spent_date FROM time_entries WHERE id='${id(6)}'`)).toBe('2027-10-07');
          } else {
            const field = kind === 'resize' ? 'minutes' : 'start_minute';
            await expect.poll(() => Number(sql(`SELECT ${field} FROM time_entries WHERE id='${id(4)}'`))).toBeGreaterThan(kind === 'resize' ? 60 : 540);
          }
          await expect(page.getByRole('alert')).toHaveCount(0);
          await expect(page.getByRole('dialog')).toHaveCount(0);
          await expect(page.getByRole('button', { name: 'Add entry', exact: true })).toBeDisabled();
        });
        await page.keyboard.press('Escape');
      }
    } finally {
      // Finish intercepted refreshes before the next case navigates away.
      await page.unrouteAll({ behavior: 'wait' });
      sql(`UPDATE projects SET active=true WHERE id='${id(2)}'`);
    }
    await check('day-view timer failure is visible', async () => {
      await open('day');
      await page.route(pattern, route => {
        assert.equal(command(route.request()).operation, 'start_timer');
        return route.abort('failed');
      });
      try {
        const rejected = page.waitForEvent('requestfailed', r => r.url().includes('/api/apply_timesheet_command'));
        const refreshed = page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
        await page.locator('.ts-day-entry').filter({ hasText: projectName }).getByRole('button', { name: 'Start', exact: true }).click();
        await rejected;
        await expect(page.getByRole('alert')).toContainText('Could not start timer');
        await (await refreshed).finished();
      } finally { await page.unroute(pattern); }
    });
    assert.deepEqual(pageErrors, []);
    assert.deepEqual(failures, []);
  } finally {
    await browser.close();
    if (created) sql(`BEGIN;
      DELETE FROM time_entries WHERE project_id = '${id(2)}';
      DELETE FROM assignments WHERE id = '${id(7)}';
      DELETE FROM project_tasks WHERE project_id = '${id(2)}';
      DELETE FROM projects WHERE id = '${id(2)}';
      DELETE FROM tasks WHERE id = '${id(3)}';
      DELETE FROM clients WHERE id = '${id(1)}';
      COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
