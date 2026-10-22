// Legacy Time report totals per currency, exclusively in the runner's disposable DB.
// Needs the freshly seeded EUR (Acme) and USD (TechStart) data: later suites such as
// invoice-preparation rewrite those projects, so the runner's default list runs this
// suite first. A focused run must name it first after a fresh migrate and seed.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const expect = baseExpect.configure({ timeout: 15000 });
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
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id,org_role FROM users WHERE email='admin@example.com' AND active) u"));
assert.equal(actor.org_role, 'admin');
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${actor.org_id}'`), '0');
// The demo seed writes the current week, so the period comes from its entries.
const seeded = `FROM time_entries te JOIN projects p ON p.id=te.project_id JOIN clients c ON c.id=p.client_id
  WHERE te.org_id='${actor.org_id}' AND c.name IN ('Acme Corp','TechStart Inc')`;
const [from, to] = sql(`SELECT min(te.spent_date)||'|'||max(te.spent_date) ${seeded}`).split('|');
assert.match(from, /^\d{4}-\d{2}-\d{2}$/);
const memberCost = '019f3100-0000-7000-8000-000000000001';
const fixtureAssignment = '019f3100-0000-7000-8000-000000000002';

// "EUR 1,130.00" → ['EUR', 113000n]; integer arithmetic only.
const parse = text => {
  const match = text.trim().match(/^(\S{3}) (-?)([\d,]+)\.(\d{2})$/);
  assert.ok(match, `Not a money amount: ${JSON.stringify(text)}`);
  const cents = BigInt(match[3].replaceAll(',', '') + match[4]);
  return [match[1], match[2] ? -cents : cents];
};
const format = (code, cents) => {
  const abs = cents < 0n ? -cents : cents;
  const whole = (abs / 100n).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return `${code} ${cents < 0n ? '-' : ''}${whole}.${(abs % 100n).toString().padStart(2, '0')}`;
};

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  let release;
  // Set only once the fixture transaction has committed; cleanup depends on it.
  let fixtureCommitted = false;
  // Resolves once the report for the given arguments has arrived.
  const reportFor = predicate => page.waitForResponse(response => {
    if (!new URL(response.url()).pathname.startsWith('/api/report_time') || !response.ok()) return false;
    try { return predicate(JSON.parse(response.request().postData() || '{}')); } catch { return false; }
  });
  const settled = async () => {
    await expect(page.getByText('Loading report…', { exact: true })).toHaveCount(0);
    await expect(totalRow).toHaveCount(1);
  };
  const select = label => page.locator('.form-group', { has: page.locator('label', { hasText: new RegExp(`^${label}$`) }) }).locator('select');
  const totalRow = page.locator('tr.report-total-row');
  const dataRows = page.locator('tbody tr:not(.report-total-row)');
  const totalLines = async column => totalRow.locator(`td:nth-child(${column}) > div`).allInnerTexts();
  // Expected total lines for one money column, summed from the loaded row cells.
  const expectedLines = async column => {
    const sums = new Map();
    for (const text of await dataRows.locator(`td:nth-child(${column})`).allInnerTexts()) {
      if (text.trim() === 'Restricted') return ['Restricted'];
      const [code, cents] = parse(text);
      sums.set(code, (sums.get(code) ?? 0n) + cents);
    }
    return [...sums.keys()].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0)).map(code => format(code, sums.get(code)));
  };
  const assertTotals = async context => {
    await expect(totalRow).toHaveCount(1);
    for (const column of [5, 6]) {
      const lines = await totalLines(column);
      assert.deepEqual(lines, await expectedLines(column), `${context}: column ${column}`);
      assert.ok(!lines.some(line => line.includes('—')), `${context}: no em dash while rows are loaded`);
    }
    return totalLines(5);
  };
  // Base and enlarged text (root font size 32px, the existing 200% text check)
  // in both themes, for both money columns. Page overflow is recorded with its
  // sources, so a pre-existing shell overflow is reported as such rather than
  // hidden; the assertions cover what this change adds: the total lines, laid
  // out in their cells and actually on screen once their column is scrolled to.
  const inspectTotals = async (phase, billing, cost) => {
    const overflow = [];
    const original = await page.evaluate(() => [document.documentElement.dataset.theme ?? null, document.documentElement.style.fontSize]);
    for (const [width, height] of [[1440, 900], [390, 844], [320, 640]]) {
      for (const theme of ['light', 'dark']) {
        for (const fontSize of ['', '32px']) {
          await page.setViewportSize({ width, height });
          await page.evaluate(([theme, fontSize]) => {
            document.documentElement.dataset.theme = theme;
            document.documentElement.style.fontSize = fontSize;
          }, [theme, fontSize]);
          await page.evaluate(() => new Promise(requestAnimationFrame));
          const label = `${phase} ${width}px ${theme}${fontSize ? ` text ${fontSize}` : ''}`;
          assert.deepEqual(await totalLines(5), billing, label);
          assert.deepEqual(await totalLines(6), cost, label);
          const rows = [];
          for (const [column, name] of [[5, 'billable'], [6, 'cost']]) {
            const geometry = await totalRow.locator(`td:nth-child(${column})`).evaluate(cell => {
              const box = el => el.getBoundingClientRect();
              const scroller = cell.closest('.table-container');
              const lines = [...cell.children].map(line => {
                const range = document.createRange();
                range.selectNodeContents(line);
                const text = [...range.getClientRects()];
                return {
                  align: getComputedStyle(line).textAlign,
                  clipped: line.scrollWidth > line.clientWidth + 1,
                  textInside: text.every(r => r.left >= box(cell).left - 1 && r.right <= box(cell).right + 1),
                  textRows: new Set(text.map(r => Math.round(r.top))).size,
                  top: box(line).top, bottom: box(line).bottom,
                };
              });
              const page = document.documentElement.scrollWidth - innerWidth;
              const sources = page > 1
                ? [...document.querySelectorAll('body *')]
                  .filter(el => el.getBoundingClientRect().right > innerWidth + 1 && !el.closest('.table-container'))
                  .slice(0, 5).map(el => `${el.tagName.toLowerCase()}.${[...el.classList].join('.')}`)
                : [];
              return { lines, scrollerFits: box(scroller).right <= innerWidth + 1, page, sources };
            });
            assert.ok(geometry.scrollerFits, `${label}: the table scrolls inside the viewport`);
            geometry.lines.forEach((line, index) => {
              assert.equal(line.align, 'right', `${label}: ${name} line ${index} right-aligned`);
              assert.equal(line.clipped, false, `${label}: ${name} line ${index} not clipped`);
              assert.ok(line.textInside, `${label}: ${name} line ${index} text within its cell`);
              if (index) assert.ok(line.top >= geometry.lines[index - 1].bottom - 1, `${label}: ${name} lines stacked in order`);
            });
            if (column === 5 && geometry.page > 1) overflow.push(`${label}: page +${geometry.page}px from ${geometry.sources.join(', ') || 'unknown'}`);
            rows.push(`${name} ${geometry.lines.map(line => line.textRows).join(',')}`);
          }
          const shot = suffix => process.env.HORAE_BROWSER_ARTIFACTS && page.screenshot({ animations: 'disabled', fullPage: suffix === '', path: join(process.env.HORAE_BROWSER_ARTIFACTS, `report-currency-totals-${phase}-${width}-${theme}${fontSize ? '-text32' : ''}${suffix}.png`) });
          if (process.env.HORAE_BROWSER_ARTIFACTS) mkdirSync(process.env.HORAE_BROWSER_ARTIFACTS, { recursive: true });
          await shot('');
          // A total inside an off-screen part of the scroller is not readable:
          // bring each money column into view and hit-test every line on screen.
          for (const [column, name] of [[5, 'billable'], [6, 'cost']]) {
            const visible = await totalRow.locator(`td:nth-child(${column})`).evaluate(cell => {
              cell.lastElementChild.scrollIntoView({ block: 'center', inline: 'end' });
              const view = cell.closest('.table-container').getBoundingClientRect();
              return [...cell.children].map(line => {
                const range = document.createRange();
                range.selectNodeContents(line);
                const r = range.getBoundingClientRect();
                const inside = r.left >= Math.max(view.left, 0) - 1 && r.right <= Math.min(view.right, innerWidth) + 1
                  && r.top >= 0 && r.bottom <= innerHeight;
                const hit = document.elementFromPoint((r.left + r.right) / 2, (r.top + r.bottom) / 2);
                return { text: line.textContent, inside, hit: !!hit && line.contains(hit) };
              });
            });
            for (const line of visible) assert.ok(line.inside && line.hit, `${label}: ${name} total "${line.text}" is on screen and unobscured`);
            await shot(`-${name}-scrolled`);
          }
          await page.evaluate(() => {
            document.querySelector('tr.report-total-row').closest('.table-container').scrollLeft = 0;
            window.scrollTo(0, 0);
          });
          console.log(`${label}: text rows per total line: ${rows.join('; ')}`);
        }
      }
    }
    await page.evaluate(([theme, fontSize]) => {
      if (theme === null) delete document.documentElement.dataset.theme;
      else document.documentElement.dataset.theme = theme;
      document.documentElement.style.fontSize = fontSize;
    }, original);
    await page.setViewportSize({ width: 1440, height: 900 });
    if (overflow.length) console.log(`Page overflow outside the report table, ${phase} (compare with the base build):\n${overflow.join('\n')}`);
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: /Sign in as Admin/ }).click();
    await page.waitForURL(url => !url.pathname.startsWith('/auth/'));
    await page.goto(`${base}/reports`);
    await page.locator('input[type="date"]').first().fill(from);
    await page.locator('input[type="date"]').last().fill(to);
    await expect(totalRow).toHaveCount(1);

    // The seeded EUR and USD clients each get their own exact total line.
    const billable = await assertTotals('project grouping');
    assert.ok(billable.some(line => line.startsWith('EUR ')) && billable.some(line => line.startsWith('USD ')), `Both seeded currencies: ${billable}`);
    assert.equal((await totalLines(6)).length, 1, 'Cost stays in the organization currency');
    // Export links are untouched by this change: exact hrefs as before.
    const exportHrefs = async () => Promise.all(['CSV', 'XLSX'].map(format =>
      page.getByRole('link', { name: `Export ${format}`, exact: true }).getAttribute('href')));
    assert.deepEqual(await exportHrefs(), ['csv', 'xlsx'].map(format => `/api/reports/export/${format}?from=${from}&to=${to}`));

    // Each grouping is checked against its own response, not the previous one.
    for (const group of ['client', 'task', 'person', 'project']) {
      const loaded = reportFor(args => args.group_by === group);
      await select('Group by').selectOption(group);
      await loaded;
      await settled();
      if (group === 'client') await expect(dataRows.locator('td:first-child', { hasText: /^Acme Corp$/ })).toHaveCount(1);
      assert.deepEqual(await assertTotals(`${group} grouping`), billable);
    }
    const acme = sql(`SELECT id FROM clients WHERE org_id='${actor.org_id}' AND name='Acme Corp'`);
    let loaded = reportFor(args => args.client_id === acme);
    await select('Client').selectOption(acme);
    await loaded;
    await settled();
    // Acme bills in EUR only, so the filtered total has a single line.
    assert.deepEqual((await assertTotals('client filter')).map(line => line.slice(0, 4)), ['EUR ']);
    assert.deepEqual(await exportHrefs(), ['csv', 'xlsx'].map(format => `/api/reports/export/${format}?from=${from}&to=${to}&client_id=${acme}`));
    loaded = reportFor(args => args.client_id === null);
    await select('Client').selectOption('');
    await loaded;
    await settled();
    assert.deepEqual(await assertTotals('all clients'), billable);

    // Loading hides the total row; a failed load offers a retry that restores it.
    // Every intercepted request waits on one shared gate. Opening it once also
    // lets through any handler that is still fetching, so unrouting (which
    // waits for handlers) cannot hang on a late arrival.
    let fetched = 0;
    const gate = new Promise(resolve => { release = resolve; });
    await page.route('**/api/report_time*', async route => {
      const response = await route.fetch();
      fetched += 1;
      await gate;
      await route.fulfill({ response });
    });
    await select('Group by').selectOption('client');
    await expect.poll(() => fetched > 0).toBe(true);
    await expect(page.getByRole('status')).toHaveText('Loading report…');
    await expect(totalRow).toHaveCount(0);
    loaded = reportFor(args => args.group_by === 'client');
    release();
    release = undefined;
    await page.unrouteAll({ behavior: 'wait' });
    await loaded;
    await settled();
    assert.deepEqual(await assertTotals('after a delayed load'), billable);
    await page.route('**/api/report_time*', route => route.fulfill({ status: 500, body: 'fixture failure' }));
    await select('Group by').selectOption('project');
    const retry = page.getByRole('button', { name: 'Retry report', exact: true });
    await expect(retry).toBeVisible();
    await expect(totalRow).toHaveCount(0);
    await page.unroute('**/api/report_time*');
    loaded = reportFor(args => args.group_by === 'project');
    await retry.focus();
    await page.keyboard.press('Enter');
    await loaded;
    await settled();
    assert.deepEqual(await assertTotals('after retry'), billable);
    assert.equal(await totalRow.locator('a, button, input, select, [tabindex]').count(), 0, 'Totals add nothing to the tab order');

    await inspectTotals('admin', billable, await totalLines(6));

    // A real manager session with an administrator-only cost override.
    const [project, user] = sql(`SELECT te.project_id||'|'||te.user_id ${seeded}
      AND te.spent_date BETWEEN '${from}' AND '${to}' ORDER BY te.id LIMIT 1`).split('|');
    assert.match(project, /^[0-9a-f-]{36}$/);
    assert.match(user, /^[0-9a-f-]{36}$/);
    // Cleanup deletes by these IDs, so they must not already belong to anything.
    assert.equal(sql(`SELECT count(*) FROM project_member_costs WHERE id='${memberCost}' OR (project_id='${project}' AND user_id='${user}')`), '0');
    assert.equal(sql(`SELECT count(*) FROM assignments WHERE id='${fixtureAssignment}'`), '0');
    const fixture = { assignment: sql(`SELECT count(*) FROM assignments WHERE project_id='${project}' AND user_id='${user}'`) === '0' };
    sql(`BEGIN;
      ${fixture.assignment ? `INSERT INTO assignments (id,project_id,user_id) VALUES ('${fixtureAssignment}','${project}','${user}');` : ''}
      INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ('${memberCost}','${actor.org_id}','${project}','${user}',9999);
      UPDATE users SET org_role='manager' WHERE id='${actor.id}'; COMMIT;`);
    fixtureCommitted = true;
    await page.reload();
    await page.locator('input[type="date"]').first().fill(from);
    await page.locator('input[type="date"]').last().fill(to);
    await expect(totalRow).toHaveCount(1);
    await expect.poll(() => totalLines(6)).toEqual(['Restricted']);
    assert.deepEqual(await totalLines(5), billable, 'Billable totals are not withheld');
    await assertTotals('manager with a withheld cost');
    await inspectTotals('restricted', billable, ['Restricted']);

    assert.deepEqual(errors, []);
    console.log(`PASS: legacy report currency totals ${billable.join(' / ')}, groupings, filter, loading, retry, both themes with base and 32px root text at 1440/390/320px, and withheld cost; Chromium ${browser.version()}`);
  } finally {
    // An intercepted request still waiting on release would block unrouteAll.
    if (release) release();
    release = undefined;
    try {
      await page.unrouteAll({ behavior: 'wait' });
    } finally {
      try {
        await browser.close();
      } finally {
        // A failed precondition never committed, so it must not delete anything.
        if (fixtureCommitted) sql(`BEGIN;
          UPDATE users SET org_role='${actor.org_role}' WHERE id='${actor.id}';
          DELETE FROM project_member_costs WHERE id='${memberCost}';
          DELETE FROM assignments WHERE id='${fixtureAssignment}'; COMMIT;`);
      }
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
