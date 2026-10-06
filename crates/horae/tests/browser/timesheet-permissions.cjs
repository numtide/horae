// Real selected-person commands, only in the runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = '01950000-0000-7000-8000-000000000002';
const org = '01950000-0000-7000-8000-000000000001';
const id = n => `019f2900-0000-7000-8000-${String(n).padStart(12, '0')}`;
const person = id(1), emptyPerson = id(2), client = id(3), project = id(4), hiddenProject = id(5), task = id(6);
const entry = id(7), hiddenEntry = id(8), ownTimer = id(9), managerEdge = id(10);
const personName = 'Scoped browser teammate', emptyName = 'Scoped browser empty teammate';
const projectName = 'Scoped browser project', taskName = 'Scoped browser task';
const day = sql('SELECT CURRENT_DATE');
assert.match(day, /^\d{4}-\d{2}-\d{2}$/);
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const grants = writable => [...floor, 'time_read_managed', ...(writable ? ['time_write_managed'] : [])];
const array = values => `ARRAY[${values.map(value => `'${value}'`).join(',')}]`;
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${org}'`), '0');
assert.equal(sql("SELECT count(*) FROM users WHERE active AND org_role='admin'"), '1');
const originalRevision = sql(`SELECT access_revision FROM organizations WHERE id='${org}'`);
assert.match(originalRevision, /^\d+$/);
function setWrite(writable) {
  sql(`BEGIN; UPDATE organizations SET access_revision=access_revision+1 WHERE id='${org}';
    UPDATE person_permission_states SET grants=${array(grants(writable))}, revision=revision+1 WHERE org_id='${org}' AND user_id='${actor}'; COMMIT;`);
}
function assignPerson() {
  sql(`INSERT INTO assignments (id,project_id,user_id) VALUES ('${id(13)}','${project}','${person}')`);
}

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [], requests = [], pending = new Set();
  let created = false, commandEndpoint;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('response', response => {
    if (response.url().includes('/api/load_timesheet_page') && response.status() !== 200) {
      console.error('Timesheet read failed:', response.status(), response.url());
    }
  });
  page.on('request', request => {
    if (new URL(request.url()).pathname.startsWith('/api/')) pending.add(request);
    if (request.url().includes('/api/apply_timesheet_command')) {
      commandEndpoint = request.url();
      requests.push(request.postDataJSON());
    }
  });
  page.on('requestfinished', request => pending.delete(request));
  page.on('requestfailed', request => pending.delete(request));
  const settled = () => expect.poll(() => pending.size).toBe(0);
  const sheetResponse = () => page.waitForResponse(r => r.url().includes('/api/load_timesheet_page') && r.status() === 200);
  const expectedContext = user_id => ({ expected_requester: { org_id: org, user_id }, subject_id: person, expected_policy: 'scoped' });
  const visit = async (mode, subject = person, anchor = day) => {
    await settled();
    const ready = sheetResponse();
    await page.goto(`${base}/timesheet/${mode}/${anchor}?span=week${subject ? `&user=${subject}` : ''}`);
    const data = await (await ready).json();
    await settled();
    assert.equal(data.policy, 'scoped');
    assert.equal(data.subject.id, subject || actor);
    assert.equal(data.next_after, null);
    return data;
  };
  const row = page.locator('.ts-day-entry').filter({ hasText: 'Delegated seed' });
  const dialog = page.getByRole('dialog', { name: /Edit time entry/ });
  const fixtureMinutes = () => Number(sql(`SELECT minutes FROM time_entries WHERE id='${entry}'`));
  const mutation = async (operation, action) => {
    const response = page.waitForResponse(r => r.url().includes('/api/apply_timesheet_command'));
    await action();
    const result = await response;
    assert.equal(result.status(), 200, await result.text());
    assert.equal(requests.at(-1).command.operation, operation);
    assert.deepEqual(requests.at(-1).context, expectedContext(actor));
    await settled();
  };
  try {
    sql(`BEGIN;
      INSERT INTO users (id,org_id,email,name) VALUES
        ('${person}','${org}','scoped-timesheet@example.test','${personName}'),
        ('${emptyPerson}','${org}','scoped-timesheet-empty@example.test','${emptyName}');
      INSERT INTO clients (id,org_id,name,currency) VALUES ('${client}','${org}','Scoped browser client','EUR');
      INSERT INTO projects (id,org_id,client_id,name,currency) VALUES
        ('${project}','${org}','${client}','${projectName}','EUR'),
        ('${hiddenProject}','${org}','${client}','Unmanaged secret project','EUR');
      INSERT INTO tasks (id,org_id,name) VALUES ('${task}','${org}','${taskName}');
      INSERT INTO project_tasks (project_id,task_id,billable) VALUES ('${project}','${task}',true),('${hiddenProject}','${task}',true);
      INSERT INTO assignments (id,project_id,user_id) VALUES
        ('${id(12)}','${project}','${actor}'),('${id(13)}','${project}','${person}'),
        ('${id(14)}','${project}','${emptyPerson}'),('${id(15)}','${hiddenProject}','${person}');
      INSERT INTO project_management_assignments (id,org_id,manager_id,project_id)
        VALUES ('${managerEdge}','${org}','${actor}','${project}');
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES
        ('${id(16)}','${org}','${actor}',1,${array(grants(true))},false,'individual'),
        ('${id(17)}','${org}','${person}',1,${array(floor)},false,'individual'),
        ('${id(18)}','${org}','${emptyPerson}',1,${array(floor)},false,'individual');
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,notes,billable,start_minute) VALUES
        ('${entry}','${org}','${person}','${project}','${task}','${day}',60,'Delegated seed',true,540),
        ('${hiddenEntry}','${org}','${person}','${hiddenProject}','${task}','${day}',1200,'Unmanaged secret notes',true,NULL),
        ('${ownTimer}','${org}','${actor}','${project}','${task}','${day}',0,'Requester timer',true,NULL);
      UPDATE time_entries SET is_running=true,started_at=now() WHERE id='${ownTimer}';
      UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);
    created = true;
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await visit('day', null);
    await page.getByRole('button', { name: 'Timesheet for', exact: true }).click();
    const picker = page.getByRole('dialog', { name: 'Choose person', exact: true });
    await expect(picker.getByRole('option', { name: emptyName, exact: true })).toBeVisible();
    const selected = sheetResponse();
    await picker.getByRole('option', { name: personName, exact: true }).click();
    const visible = await (await selected).json();
    assert.deepEqual(visible.entries.map(value => value.id), [entry]);
    assert.equal(visible.subject.id, person);
    await settled();
    await expect(page.locator('#app-main')).not.toContainText('Unmanaged secret');
    await expect(row.locator('.ts-day-entry-dur')).toHaveText('1:00');
    console.log('PASS: project scope includes zero-entry active teammates without exposing unrelated hours');

    await page.getByRole('button', { name: 'Timesheet for', exact: true }).click();
    for (const [subject, action] of [
      [emptyPerson, () => picker.getByRole('option', { name: emptyName, exact: true }).click()],
      [person, () => page.goBack()],
      [emptyPerson, () => page.goForward()],
      [person, () => page.goBack()],
    ]) {
      const changed = sheetResponse();
      await action();
      const data = await (await changed).json();
      assert.equal(data.subject.id, subject);
      assert.deepEqual(data.entries.map(value => value.id), subject === person ? [entry] : []);
      await expect(page.getByRole('button', { name: 'Timesheet for', exact: true })).toContainText(subject === person ? personName : emptyName);
      await expect(row).toHaveCount(subject === person ? 1 : 0);
      await settled();
    }
    console.log('PASS: changing person and browser history reload the correct sheet, including an empty teammate');

    await row.getByRole('button', { name: 'Edit', exact: true }).click();
    await dialog.getByRole('textbox', { name: 'Duration', exact: true }).fill('1:30');
    await mutation('update', () => dialog.getByRole('button', { name: 'Save entry', exact: true }).click());
    assert.equal(fixtureMinutes(), 90);
    assert.equal(sql(`SELECT user_id FROM time_entries WHERE id='${entry}'`), person);
    await expect(row.locator('.ts-day-entry-dur')).toHaveText('1:30');
    await page.getByRole('button', { name: 'Week', exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`user=${person}`));
    const cell = page.getByRole('textbox', { name: `Hours for ${projectName}, ${taskName}, ${day}`, exact: true });
    await expect(cell).toHaveValue('1:30');
    await cell.fill('2:00');
    await mutation('update', () => cell.press('Tab'));
    assert.equal(fixtureMinutes(), 120);
    assert.equal(sql(`SELECT minutes FROM time_entries WHERE id='${hiddenEntry}'`), '1200');
    console.log('PASS: Day and Week edits carry the selected owner and preserve hidden work');

    // Separate week keeps calendar mutations independent of the timer fixture.
    const calendarDay = '2027-10-04', nextDay = '2027-10-05';
    const createdEntry = notes => {
      const value = sql(`SELECT id FROM time_entries WHERE project_id='${project}' AND notes='${notes}'`);
      assert.match(value, /^[0-9a-f-]{36}$/);
      return value;
    };
    const stored = entryId => JSON.parse(sql(`SELECT json_build_object('user',user_id,'project',project_id,'task',task_id,
      'day',spent_date,'minutes',minutes,'start',start_minute,'order',sort_order) FROM time_entries WHERE id='${entryId}'`));
    const expectedEntry = (spentDay, minutes, start, order = 0) => ({ user: person, project, task, day: spentDay, minutes, start, order });
    const adding = page.getByRole('dialog', { name: /New time entry/ });
    for (const [date, duration, notes] of [
      [calendarDay, '1:00', 'Delegated untimed'], [nextDay, '0:30', 'Delegated sibling'],
    ]) {
      await visit('day', person, date);
      await page.getByRole('button', { name: 'Add entry', exact: true }).click();
      await expect(adding.locator('#time-entry-title')).toHaveText(`New time entry for ${date === calendarDay ? 'Monday, 04' : 'Tuesday, 05'} Oct`);
      await expect(adding.getByRole('button', { name: 'Project', exact: true })).toContainText(projectName);
      await expect(adding.getByRole('button', { name: 'Task', exact: true })).toContainText(taskName);
      await adding.getByRole('textbox', { name: 'Duration', exact: true }).fill(duration);
      await adding.getByPlaceholder('Notes (optional)').fill(notes);
      await mutation('create', () => adding.getByRole('button', { name: 'Save entry', exact: true }).click());
      await expect(page.locator('.ts-day-entry').filter({ hasText: notes })).toBeVisible();
    }
    const untimedId = createdEntry('Delegated untimed');
    const siblingId = createdEntry('Delegated sibling');
    assert.deepEqual(stored(untimedId), expectedEntry(calendarDay, 60, null));
    assert.deepEqual(stored(siblingId), expectedEntry(nextDay, 30, null));
    console.log('PASS: delegated creation uses the selected person and eligible project/task');

    await page.getByRole('button', { name: 'Calendar', exact: true }).click();
    await page.locator('#calendar-span-menu-trigger').click();
    await page.getByRole('menuitem', { name: 'Day view', exact: true }).click();
    await expect(page.locator('.ts-cal-col')).toHaveCount(1);
    await page.getByRole('button', { name: 'Add entry', exact: true }).click();
    await expect(adding.locator('#time-entry-title')).toHaveText('New time entry for Tuesday, 05 Oct');
    await adding.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.locator('#calendar-span-menu-trigger').click();
    await page.getByRole('menuitem', { name: 'Week view', exact: true }).click();
    const columns = page.locator('.ts-cal-col');
    await expect(columns).toHaveCount(7);
    await expect(columns.nth(0).locator('.ts-cal-event')).not.toHaveClass(/locked/);
    const hourHeight = await page.locator('.ts-cal-hour').first().evaluate(el => el.getBoundingClientRect().height);
    assert.ok(hourHeight > 0);
    const monday = await columns.nth(0).boundingBox();
    const drawX = monday.x + monday.width / 2, drawY = monday.y + hourHeight * 3 + 2;
    await page.mouse.move(drawX, drawY);
    await page.mouse.down();
    await page.mouse.move(drawX, drawY + hourHeight, { steps: 5 });
    await page.mouse.up();
    await expect(adding).toBeVisible();
    await expect(adding.getByRole('textbox', { name: 'Duration', exact: true })).toHaveValue('1:00');
    await adding.getByPlaceholder('Notes (optional)').fill('Delegated timed');
    await mutation('create', () => adding.getByRole('button', { name: 'Save entry', exact: true }).click());
    const timedId = createdEntry('Delegated timed');
    assert.deepEqual(stored(timedId), expectedEntry(calendarDay, 60, 180));
    const timed = page.locator('.ts-cal-event.timed');
    await expect(timed.locator('.ts-cal-ev-time')).toHaveText('3:00–4:00');

    const drag = async (source, targetX, deltaY) => {
      await source.scrollIntoViewIfNeeded();
      const box = await source.boundingBox();
      assert.ok(box);
      const x = box.x + box.width / 2, y = box.y + box.height / 2;
      await page.mouse.move(x, y);
      await page.mouse.down();
      await page.mouse.move(targetX ?? x, y + deltaY, { steps: 5 });
      await page.mouse.up();
    };
    const tuesday = await columns.nth(1).boundingBox();
    await mutation('reschedule', () => drag(timed, tuesday.x + tuesday.width / 2, hourHeight));
    assert.deepEqual(stored(timedId), expectedEntry(nextDay, 60, 240));
    await expect(columns.nth(1).locator('.ts-cal-ev-time')).toHaveText('4:00–5:00');
    await expect(timed).not.toHaveClass(/locked/);
    await mutation('reschedule', () => drag(timed.locator('.ts-cal-resize'), null, hourHeight / 2));
    assert.deepEqual(stored(timedId), expectedEntry(nextDay, 90, 240));
    await expect(timed.locator('.ts-cal-ev-time')).toHaveText('4:00–5:30');
    console.log('PASS: delegated Calendar drawing, day movement and resizing persist exact minutes');

    const untimed = columns.nth(0).locator('.ts-cal-event:not(.timed)');
    await expect(untimed).not.toHaveClass(/locked/);
    await mutation('reorder', () => drag(untimed, tuesday.x + tuesday.width / 2, 30));
    assert.deepEqual(requests.at(-1).command.entry_ids, [siblingId, untimedId]);
    assert.deepEqual(stored(siblingId), expectedEntry(nextDay, 30, null));
    assert.deepEqual(stored(untimedId), expectedEntry(nextDay, 60, null, 1));
    await expect(columns.nth(0).locator('.ts-cal-event')).toHaveCount(0);
    await expect(columns.nth(1).locator('.ts-cal-event')).toHaveCount(3);
    const moved = columns.nth(1).locator('.ts-cal-event:not(.timed)').filter({ has: page.locator('.ts-cal-ev-dur', { hasText: '1:00' }) });
    await expect(moved).not.toHaveClass(/locked/);
    await mutation('reorder', () => drag(moved, null, -hourHeight));
    assert.deepEqual(requests.at(-1).command.entry_ids, [untimedId, siblingId]);
    assert.deepEqual(stored(untimedId), expectedEntry(nextDay, 60, null));
    assert.deepEqual(stored(siblingId), expectedEntry(nextDay, 30, null, 1));
    console.log('PASS: delegated untimed moves and order inversions persist the complete ordered entry set');

    await visit('day', person, nextDay);
    await page.locator('.ts-day-entry').filter({ hasText: 'Delegated timed' }).getByRole('button', { name: 'Edit', exact: true }).click();
    await mutation('delete', () => dialog.getByRole('button', { name: 'Delete', exact: true }).click());
    assert.equal(sql(`SELECT count(*) FROM time_entries WHERE id='${timedId}'`), '0');
    await expect(page.locator('.ts-day-entry').filter({ hasText: 'Delegated timed' })).toHaveCount(0);
    assert.deepEqual(stored(untimedId), expectedEntry(nextDay, 60, null));
    await page.getByRole('button', { name: 'Week', exact: true }).click();
    const createdRow = page.locator('.ts-body').filter({ hasText: projectName });
    await expect(createdRow.locator('.ts-rowtotal')).toHaveText('1:30');
    const beforeDelete = requests.length;
    await mutation('delete', () => createdRow.getByRole('button', { name: 'Remove row', exact: true }).click());
    assert.equal(requests.length, beforeDelete + 1);
    assert.deepEqual(requests.at(-1).command.entry_ids.slice().sort(), [untimedId, siblingId].sort());
    assert.equal(sql(`SELECT count(*) FROM time_entries WHERE id IN ('${untimedId}','${timedId}','${siblingId}')`), '0');
    await expect(page.getByRole('button', { name: 'Add entry', exact: true })).toBeEnabled();
    await expect(createdRow).toHaveCount(0);
    assert.equal(fixtureMinutes(), 120);
    assert.equal(sql(`SELECT minutes FROM time_entries WHERE id='${hiddenEntry}'`), '1200');
    assert.equal(sql(`SELECT is_running FROM time_entries WHERE id='${ownTimer}'`), 't');
    console.log('PASS: delegated modal and Week row deletion preserve unrelated hours and requester timer');

    await visit('day');
    await row.getByRole('button', { name: 'Edit', exact: true }).click();
    await dialog.getByRole('textbox', { name: 'Duration', exact: true }).fill('3:00');
    let release, intercepted = false;
    const gate = new Promise(resolve => { release = resolve; });
    const pattern = '**/api/apply_timesheet_command*';
    await page.route(pattern, async route => { intercepted = true; await gate; await route.continue(); });
    try {
      const denied = page.waitForResponse(r => r.url().includes('/api/apply_timesheet_command'));
      await dialog.getByRole('button', { name: 'Save entry', exact: true }).click();
      await expect.poll(() => intercepted).toBe(true);
      setWrite(false);
      release();
      assert.equal((await denied).status(), 403);
      await expect(dialog.getByRole('alert')).toContainText('Could not save');
      await expect(dialog.getByRole('textbox', { name: 'Duration', exact: true })).toHaveValue('3:00');
      assert.equal(fixtureMinutes(), 120);
    } finally { release(); await page.unroute(pattern); }
    await page.keyboard.press('Escape');
    await visit('day');
    await expect(row.getByRole('button', { name: 'Edit', exact: true })).toBeDisabled();
    await expect(page.getByRole('button', { name: 'Add entry', exact: true })).toBeDisabled();
    setWrite(true);
    await visit('day');
    console.log('PASS: revocation between click and commit rejects the write and preserves the draft/data');

    const shellTimer = page.waitForResponse(r => r.url().includes('/api/get_current_timer') && r.status() === 200);
    await mutation('start_timer', () => row.getByRole('button', { name: 'Start', exact: true }).click());
    assert.equal((await (await shellTimer).json()).id, ownTimer);
    const timerId = sql(`SELECT id FROM time_entries WHERE user_id='${person}' AND is_running`);
    assert.match(timerId, /^[0-9a-f-]{36}$/);
    // Timer commands use the server's current date, not the sheet's selected date.
    const timerDay = sql(`SELECT spent_date FROM time_entries WHERE id='${timerId}'`);
    assert.match(timerDay, /^\d{4}-\d{2}-\d{2}$/);
    assert.equal(sql(`SELECT is_running FROM time_entries WHERE id='${ownTimer}'`), 't');
    await expect(page.locator('.sidebar-timer-wrap').getByRole('button', { name: 'Stop timer', exact: true })).toBeVisible();
    sql(`DELETE FROM assignments WHERE id='${id(13)}'`);
    const withoutAssignment = await visit('day', person, timerDay);
    assert.ok(withoutAssignment.entries.some(value => value.id === timerId && value.is_running));
    const runningRow = page.locator('.ts-day-entry').filter({ has: page.getByRole('button', { name: 'Stop', exact: true }) });
    await expect(runningRow.getByRole('button', { name: 'Stop', exact: true })).toBeDisabled();
    const refused = await context.request.post(commandEndpoint, { data: { context: expectedContext(actor), command: { operation: 'stop_timer', entry_id: timerId } } });
    assert.equal(refused.status(), 409);
    assert.match(await refused.text(), /The selected person cannot currently track this project\/task/);
    assert.equal(sql(`SELECT is_running FROM time_entries WHERE id='${timerId}'`), 't');
    assignPerson();
    await visit('day', person, timerDay);
    await mutation('stop_timer', () => runningRow.getByRole('button', { name: 'Stop', exact: true }).click());
    assert.equal(sql(`SELECT is_running FROM time_entries WHERE id='${timerId}'`), 'f');
    await expect(page.locator('.sidebar-timer-wrap').getByRole('button', { name: 'Stop timer', exact: true })).toBeVisible();
    console.log('PASS: delegated timer leaves the requester timer alone and requires restored tracking eligibility');

    // Owner recovery uses a second real session, not a spoofed requester payload.
    sql(`UPDATE time_entries SET is_running=true,started_at=now() WHERE id='${timerId}';
      DELETE FROM assignments WHERE id='${id(13)}';
      UPDATE users SET org_role='member' WHERE id='${actor}';
      UPDATE users SET org_role='admin' WHERE id='${person}';`);
    const owner = await browser.newPage();
    const ownerPending = new Set();
    owner.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) ownerPending.add(request); });
    owner.on('requestfinished', request => ownerPending.delete(request));
    owner.on('requestfailed', request => ownerPending.delete(request));
    const ownerSettled = () => expect.poll(() => ownerPending.size).toBe(0);
    owner.on('pageerror', error => errors.push(error.stack || error.message));
    await owner.goto(`${base}/auth/login`);
    await owner.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await owner.waitForURL(`${base}/`);
    await ownerSettled();
    sql(`UPDATE users SET org_role='admin' WHERE id='${actor}'; UPDATE users SET org_role='member' WHERE id='${person}';`);
    await owner.goto(`${base}/timesheet/day/${timerDay}?span=week`);
    const ownRow = owner.locator('.ts-day-entry').filter({ has: owner.getByRole('button', { name: 'Stop', exact: true }) });
    await expect(ownRow.getByRole('button', { name: 'Edit', exact: true })).toBeDisabled();
    const stopped = owner.waitForResponse(r => r.url().includes('/api/apply_timesheet_command'));
    await ownRow.getByRole('button', { name: 'Stop', exact: true }).click();
    const stoppedResponse = await stopped;
    assert.equal(stoppedResponse.status(), 200, await stoppedResponse.text());
    assert.deepEqual(stoppedResponse.request().postDataJSON().context, expectedContext(person));
    assert.equal(sql(`SELECT is_running FROM time_entries WHERE id='${timerId}'`), 'f');
    assert.equal(sql(`SELECT is_running FROM time_entries WHERE id='${ownTimer}'`), 't');
    await expect(owner.locator('.ts-day-entry').getByRole('button', { name: 'Stop', exact: true })).toHaveCount(0);
    await ownerSettled();
    await owner.close();
    console.log('PASS: only the owner can use terminal recovery after losing project assignment');

    sql(`BEGIN; UPDATE organizations SET access_revision=access_revision+1 WHERE id='${org}';
      DELETE FROM project_management_assignments WHERE id='${managerEdge}'; COMMIT;`);
    await page.goto(`${base}/timesheet/day/${day}?span=week&user=${person}`);
    await expect(page.getByRole('alert')).toContainText('Could not load the complete timesheet');
    await settled();
    await expect(page.locator('.ts-day-entry')).toHaveCount(0);
    await expect(page.locator('#app-main')).not.toContainText('Delegated seed');
    assert.equal(fixtureMinutes(), 120);
    assert.deepEqual(errors, []);
    console.log('PASS: relationship revocation removes the selected sheet without falling back to own data');
  } catch (error) {
    console.error('Browser state:', page.url(), await page.locator('#app-main').innerText(), errors);
    throw error;
  } finally {
    await browser.close();
    if (created) sql(`BEGIN;
      UPDATE organizations SET permission_policy_version=0,access_revision=${originalRevision} WHERE id='${org}';
      UPDATE users SET org_role='admin' WHERE id='${actor}';
      DELETE FROM time_entries WHERE project_id IN ('${project}','${hiddenProject}');
      DELETE FROM project_management_assignments WHERE id='${managerEdge}';
      DELETE FROM assignments WHERE project_id IN ('${project}','${hiddenProject}');
      DELETE FROM project_tasks WHERE project_id IN ('${project}','${hiddenProject}');
      DELETE FROM person_permission_states WHERE id IN ('${id(16)}','${id(17)}','${id(18)}');
      DELETE FROM projects WHERE id IN ('${project}','${hiddenProject}');
      DELETE FROM tasks WHERE id='${task}'; DELETE FROM clients WHERE id='${client}';
      DELETE FROM users WHERE id IN ('${person}','${emptyPerson}'); COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
