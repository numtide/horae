// Legacy Reports screen, Export links and downloads against hand-derived
// fixture values (specs/004-invoice-timesheet-exports plan, "Acceptance
// Increment"). XLSX contents are proven by the Rust legacy_exports check for
// the same links; this suite proves the screen, both links and the CSV bytes.
// Needs the fresh seed at policy 0: run before reports-permissions.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const expect = baseExpect.configure({ timeout: 15000 });
const { execFileSync } = require('node:child_process');
const { readFileSync } = require('node:fs');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id,name FROM users WHERE email='admin@example.com' AND active AND org_role='admin') u"));
const org = actor.org_id;
const id = n => `019f3200-0000-7000-8000-${n.toString(16).padStart(12, '0')}`;
const [from, to] = ['2041-03-04', '2041-03-06'];

const clients = [[id(16), 'Export client A'], [id(17), 'Export client B']];
const projects = [[id(32), 'Export alpha', 0], [id(33), 'Export beta', 0], [id(34), 'Export gamma', 1]];
const [task, colleague] = [id(48), id(49)];
const users = [actor.name, 'Export colleague'];
// Raw and rounded (15 minutes, nearest) values as shown: hand-derived, never read back.
const entries = [
  { date: '2041-03-04', project: 0, user: 0, raw: 50, billable: true, notes: 'Legacy export, "one"', hours: '0.83', rounded: '0.75' },
  { date: '2041-03-04', project: 0, user: 1, raw: 50, billable: true, notes: 'Legacy export <two> & more', hours: '0.83', rounded: '0.75' },
  { date: '2041-03-05', project: 1, user: 0, raw: 53, billable: false, notes: 'Legacy export three', hours: '0.88', rounded: '1.00' },
  { date: '2041-03-05', project: 1, user: 0, raw: 7, billable: true, notes: '', hours: '0.12', rounded: '0.00' },
  { date: '2041-03-06', project: 2, user: 0, raw: 90, billable: true, notes: 'Legacy export excluded', hours: '1.50', rounded: '1.50' },
  { date: '2041-03-07', project: 0, user: 0, raw: 60, billable: true, notes: 'Legacy export outside', hours: '1.00', rounded: '1.00' },
].map((entry, n) => ({ ...entry, id: id(256 + n) }));
// Time tab, grouped by project: [Total hours, Rounded, Billable hrs].
const byProject = { 'Export alpha': ['1.67', '1.50', '1.50'], 'Export beta': ['1.00', '1.00', '0.00'], 'Export gamma': ['1.50', '1.50', '1.50'] };
const states = [
  { name: 'no filter', client: null, project: null, entries: [0, 1, 2, 3, 4], total: ['4.17', '4.00', '3.00'] },
  { name: 'project only', client: null, project: projects[2][0], entries: [4], total: ['1.50', '1.50', '1.50'] },
  { name: 'client only', client: clients[0][0], project: null, entries: [0, 1, 2, 3], total: ['2.67', '2.50', '1.50'] },
  { name: 'client and project', client: clients[0][0], project: projects[1][0], entries: [2, 3], total: ['1.00', '1.00', '0.00'] },
];

const row = entry => [entry.date, projects[entry.project][1], 'Export task', users[entry.user], entry.hours, entry.rounded, entry.billable ? 'Yes' : 'No'];
// Writes (does not parse) CSV: quote a field only when it holds a quote, comma or newline.
const field = value => /[",\n\r]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
const csvFor = picked => ['Date,Project,Task,User,Hours,Rounded Hours,Billable,Notes',
  ...picked.map(entry => [...row(entry), entry.notes].map(field).join(','))].map(line => `${line}\n`).join('');

assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');
assert.equal(sql(`SELECT round_minutes||'|'||round_dir FROM organizations WHERE id='${org}'`), '15|nearest');
assert.equal(sql(`SELECT count(*) FROM time_entries WHERE org_id='${org}' AND spent_date BETWEEN '2041-03-01' AND '2041-03-31'`), '0');
const fixtureIds = [...clients.map(([key]) => key), ...projects.map(([key]) => key), task, colleague, ...entries.map(entry => entry.id)].map(key => `'${key}'`).join(',');
assert.equal(sql(`SELECT (SELECT count(*) FROM clients WHERE id IN (${fixtureIds}) OR (org_id='${org}' AND name LIKE 'Export client %'))
  + (SELECT count(*) FROM projects WHERE id IN (${fixtureIds}) OR (org_id='${org}' AND name LIKE 'Export %'))
  + (SELECT count(*) FROM tasks WHERE id IN (${fixtureIds})) + (SELECT count(*) FROM users WHERE id IN (${fixtureIds}))
  + (SELECT count(*) FROM time_entries WHERE id IN (${fixtureIds}))`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  // Set only once the fixture transaction has committed; cleanup depends on it.
  let fixtureCommitted = false;
  try {
    sql(`BEGIN;
      ${clients.map(([key, name]) => `INSERT INTO clients (id,org_id,name,currency) VALUES ('${key}','${org}','${name}','EUR');`).join('\n')}
      ${projects.map(([key, name, client]) => `INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ('${key}','${org}','${clients[client][0]}','${name}','EUR');`).join('\n')}
      INSERT INTO tasks (id,org_id,name) VALUES ('${task}','${org}','Export task');
      INSERT INTO users (id,org_id,email,name) VALUES ('${colleague}','${org}','export-colleague@example.test','Export colleague');
      ${entries.map(entry => `INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,notes) VALUES
        ('${entry.id}','${org}','${entry.user ? colleague : actor.id}','${projects[entry.project][0]}','${task}','${entry.date}',${entry.raw},${entry.billable},${entry.notes ? `'${entry.notes.replaceAll("'", "''")}'` : 'NULL'});`).join('\n')}
      COMMIT;`);
    fixtureCommitted = true;

    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: /Sign in as Admin/ }).click();
    await page.waitForURL(url => !url.pathname.startsWith('/auth/'));
    await page.goto(`${base}/reports`);

    // Resolves once both the Time and Detailed reports for these filters have arrived.
    const loaded = state => Promise.all(['report_time', 'report_detailed'].map(name => page.waitForResponse(response => {
      if (!new URL(response.url()).pathname.startsWith(`/api/${name}`) || !response.ok()) return false;
      try {
        const args = JSON.parse(response.request().postData() || '{}');
        return args.from === from && args.to === to && args.client_id === state.client && args.project_id === state.project;
      } catch { return false; }
    })));
    const select = label => page.locator('.form-group', { has: page.locator('label', { hasText: new RegExp(`^${label}$`) }) }).locator('select');
    const tab = name => page.getByRole('button', { name, exact: true });
    const link = format => page.getByRole('link', { name: `Export ${format}`, exact: true });
    const cells = locator => locator.evaluateAll(rows => rows.map(tr => [...tr.querySelectorAll('td')].map(td => td.innerText.trim())));

    const verify = async state => {
      const picked = state.entries.map(n => entries[n]);
      // P1: both links carry exactly the visible period and filters.
      let query = `from=${from}&to=${to}`;
      if (state.client) query += `&client_id=${state.client}`;
      if (state.project) query += `&project_id=${state.project}`;
      assert.equal(await link('CSV').getAttribute('href'), `/api/reports/export/csv?${query}`, state.name);
      assert.equal(await link('XLSX').getAttribute('href'), `/api/reports/export/xlsx?${query}`, state.name);

      // P2: Time tab rows by project, and the total row, against the fixture.
      await tab('Time').click();
      const expectedGroups = [...new Set(picked.map(entry => projects[entry.project][1]))].sort()
        .map(name => [name, ...byProject[name]]);
      await expect.poll(async () => (await cells(page.locator('tbody tr:not(.report-total-row)')))
        .map(cellsOfRow => cellsOfRow.slice(0, 4)).sort((a, b) => a[0].localeCompare(b[0]))).toEqual(expectedGroups);
      assert.deepEqual((await cells(page.locator('tr.report-total-row')))[0].slice(0, 4), ['Total', ...state.total], state.name);

      // P3: Detailed rows in order; excluded entries absent.
      await tab('Detailed time').click();
      await expect.poll(() => cells(page.locator('tbody tr')))
        .toEqual(picked.map(entry => [...row(entry), entry.notes || '—']));
      for (const entry of entries.filter((_, n) => !state.entries.includes(n) && entries[n].notes)) {
        await expect(page.locator('tbody')).not.toContainText(entry.notes);
      }
      await tab('Time').click();

      // P4: the real CSV download equals the expected bytes. P5: the real XLSX download.
      const [csv] = await Promise.all([page.waitForEvent('download'), link('CSV').click()]);
      assert.equal(csv.suggestedFilename(), 'timesheet.csv');
      assert.equal(readFileSync(await csv.path(), 'utf8'), csvFor(picked), state.name);
      const [xlsx] = await Promise.all([page.waitForEvent('download'), link('XLSX').click()]);
      assert.equal(xlsx.suggestedFilename(), 'timesheet.xlsx');
      assert.ok(readFileSync(await xlsx.path()).subarray(0, 2).equals(Buffer.from('PK')), state.name);
    };

    let ready = loaded(states[0]);
    await page.locator('input[type="date"]').first().fill(from);
    await page.locator('input[type="date"]').last().fill(to);
    await ready;
    await verify(states[0]);

    ready = loaded(states[1]);
    await select('Project').selectOption(states[1].project);
    await ready;
    await verify(states[1]);

    ready = loaded(states[0]);
    await select('Project').selectOption('');
    await ready;
    ready = loaded(states[2]);
    await select('Client').selectOption(states[2].client);
    await ready;
    await verify(states[2]);

    await expect(select('Project').locator(`option[value="${states[3].project}"]`)).toHaveCount(1);
    await expect(select('Project').locator(`option[value="${projects[2][0]}"]`)).toHaveCount(0);
    ready = loaded(states[3]);
    await select('Project').selectOption(states[3].project);
    await ready;
    await verify(states[3]);

    assert.deepEqual(errors, []);
    console.log(`PASS: legacy report exports for ${states.map(state => state.name).join(', ')}: links, Time and Detailed rows, CSV bytes and XLSX downloads; Chromium ${browser.version()}`);
  } finally {
    try {
      await browser.close();
    } finally {
      // A failed precondition never committed, so it must not delete anything.
      if (fixtureCommitted) sql(`BEGIN;
        DELETE FROM time_entries WHERE id IN (${entries.map(entry => `'${entry.id}'`).join(',')});
        DELETE FROM projects WHERE id IN (${projects.map(([key]) => `'${key}'`).join(',')});
        DELETE FROM clients WHERE id IN (${clients.map(([key]) => `'${key}'`).join(',')});
        DELETE FROM tasks WHERE id='${task}';
        DELETE FROM users WHERE id='${colleague}'; COMMIT;`);
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
