// Legacy Time report filters: label association, reflow on narrow and enlarged
// displays, and unchanged filter behaviour. Runs only against the runner's
// disposable database. Its fixture (one long-named client and one project tag
// linked to a seeded project) is created and removed here, so suite order does
// not matter.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const expect = baseExpect.configure({ timeout: 15000 });
const { execFileSync } = require('node:child_process');
const { join } = require('node:path');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const orgId = sql("SELECT org_id FROM users WHERE email='admin@example.com' AND active");
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${orgId}'`), '0', 'legacy report policy');
const [from, to] = sql(`SELECT min(spent_date)||'|'||max(spent_date) FROM time_entries WHERE org_id='${orgId}'`).split('|');
assert.match(from, /^\d{4}-\d{2}-\d{2}$/);
const shots = process.env.HORAE_BROWSER_ARTIFACTS;
const fixture = {
  client: '019f3200-0000-7000-8000-000000000001',
  tag: '019f3200-0000-7000-8000-000000000002',
  link: '019f3200-0000-7000-8000-000000000003',
  clientName: 'Fixture client with an exceptionally long legal name for narrow report filters',
  tagName: 'Fixture tag with a long name for narrow filters',
};

// The contract order. Project tag renders here because the fixture adds a tag.
const controls = [
  ['report-filter-from', 'From'],
  ['report-filter-to', 'To'],
  ['report-filter-group', 'Group by'],
  ['report-filter-client', 'Client'],
  ['report-filter-project', 'Project'],
  ['report-filter-teammate', 'Teammate'],
  ['report-tag', 'Project tag'],
];
const sizes = [[320, 900], [390, 900], [768, 900], [1440, 900], [320, 568], [1440, 600]];

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'en-US' });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  // Set only once the fixture transaction has committed; cleanup depends on it.
  let fixtureCommitted = false;
  const control = id => page.locator(`#${id}`);
  const responseFor = (path, predicate) => page.waitForResponse(response => {
    if (!new URL(response.url()).pathname.startsWith(path) || !response.ok()) return false;
    try { return predicate(JSON.parse(response.request().postData() || '{}')); } catch { return false; }
  });
  // The wait is registered before the action and awaited together with it, so a
  // failed action never leaves a pending waiter to reject unhandled.
  const after = async (path, predicate, action) => (await Promise.all([responseFor(path, predicate), action()]))[0];
  // Neither the time nor the detailed view is still loading.
  const settled = async () => {
    await expect(page.locator('p[role="status"]', { hasText: /^Loading/ })).toHaveCount(0);
    await expect(page.locator('table, .alert, .card.p-8').first()).toBeVisible();
  };
  const display = async (width, height, theme, fontSize) => {
    await page.setViewportSize({ width, height });
    await page.evaluate(([theme, fontSize]) => {
      document.documentElement.dataset.theme = theme;
      document.documentElement.style.fontSize = fontSize;
    }, [theme, fontSize]);
    await page.evaluate(() => document.fonts.ready.then(() => new Promise(requestAnimationFrame)));
  };
  const pageOverflow = () => page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  const exportHrefs = () => Promise.all(['CSV', 'XLSX'].map(format =>
    page.getByRole('link', { name: `Export ${format}`, exact: true }).getAttribute('href')));
  const expectedHrefs = query => ['csv', 'xlsx'].map(format => `/api/reports/export/${format}?${query}`);

  try {
    const project = sql(`SELECT id FROM projects WHERE org_id='${orgId}' AND active ORDER BY id LIMIT 1`);
    assert.match(project, /^[0-9a-f-]{36}$/);
    sql(`BEGIN;
      INSERT INTO clients (id, org_id, name, currency) VALUES ('${fixture.client}', '${orgId}', '${fixture.clientName}', 'EUR');
      INSERT INTO project_tags (id, org_id, name) VALUES ('${fixture.tag}', '${orgId}', '${fixture.tagName}');
      INSERT INTO project_tag_links (id, org_id, project_id, tag_id) VALUES ('${fixture.link}', '${orgId}', '${project}', '${fixture.tag}');
      COMMIT;`);
    fixtureCommitted = true;

    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: /Sign in as Admin/ }).click();
    await page.waitForURL(url => !url.pathname.startsWith('/auth/'));
    await page.goto(`${base}/reports`);
    await settled();
    await expect(page.locator('option', { hasText: fixture.tagName })).toBeAttached();

    // Each visible label names and focuses its own control.
    for (const [id, label] of controls) {
      assert.equal(await control(id).count(), 1, `the ${label} control has id ${id}`);
      const labels = await control(id).evaluate(el => [...el.labels].map(l => l.textContent.trim()));
      assert.deepEqual(labels, [label], `${id} has exactly its visible label`);
      assert.equal(await page.getByLabel(label, { exact: true }).getAttribute('id'), id, `${label} names ${id}`);
      await page.locator(`label[for="${id}"]`).click();
      await expect(control(id)).toBeFocused();
    }
    // Tab order follows the contract; a date field takes several Tabs for its segments.
    await control('report-filter-from').focus();
    const visited = ['report-filter-from'];
    for (let step = 0; step < 40 && visited.length < controls.length; step += 1) {
      await page.keyboard.press('Tab');
      const id = await page.evaluate(() => document.activeElement?.id || '');
      if (id && id !== visited.at(-1)) visited.push(id);
    }
    assert.deepEqual(visited, controls.map(([id]) => id), 'tab order');

    await after('/api/report_time', args => args.from === from && args.to === to, async () => {
      await control('report-filter-from').fill(from);
      await control('report-filter-to').fill(to);
    });
    await settled();

    // Reflow matrix: both tabs, every size, both themes, base and enlarged text.
    for (const tab of ['Time', 'Detailed time']) {
      await page.getByRole('button', { name: tab, exact: true }).click();
      await settled();
      for (const [width, height] of sizes) {
        for (const theme of ['dark', 'light']) {
          for (const fontSize of ['', '32px']) {
            await display(width, height, theme, fontSize);
            const label = `${tab} ${width}x${height} ${theme}${fontSize ? ' text 32px' : ''}`;
            const geometry = await page.evaluate(ids => {
              const style = el => getComputedStyle(el);
              const rootPx = parseFloat(style(document.documentElement).fontSize);
              const panel = document.getElementById(ids[0]).closest('.card');
              const box = panel.getBoundingClientRect();
              // Reference values from untouched elements of the same classes.
              const probe = (tag, className, type) => {
                const el = document.createElement(tag);
                el.className = className;
                if (type) el.type = type;
                el.style.position = 'absolute';
                el.style.visibility = 'hidden';
                document.body.append(el);
                const s = style(el);
                const values = { padding: s.padding, border: s.borderTop, background: s.backgroundColor, paddingX: [s.paddingLeft, s.paddingRight] };
                el.remove();
                return values;
              };
              const card = probe('div', 'card'), input = probe('input', 'form-input', 'date');
              const panelStyle = style(panel);
              return {
                overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
                innerWidth,
                panel: { left: box.left, right: box.right, padding: panelStyle.padding, border: panelStyle.borderTop, background: panelStyle.backgroundColor },
                card,
                input,
                controls: ids.map(id => {
                  const el = document.getElementById(id), r = el.getBoundingClientRect(), s = style(el);
                  return { id, left: r.left, right: r.right, height: r.height, font: parseFloat(s.fontSize), paddingX: [s.paddingLeft, s.paddingRight] };
                }),
                expectedFont: rootPx * 0.875,
                controlHeight: rootPx * 2.75,
              };
            }, controls.map(([id]) => id));
            assert.ok(geometry.overflow <= 0, `${label}: page overflows by ${geometry.overflow}px`);
            for (const c of geometry.controls) {
              assert.ok(c.left >= -0.5 && c.right <= geometry.innerWidth + 0.5, `${label}: ${c.id} inside the viewport (${c.left}..${c.right})`);
              assert.ok(c.left >= geometry.panel.left - 0.5 && c.right <= geometry.panel.right + 0.5, `${label}: ${c.id} inside the filter panel`);
              assert.equal(c.font, geometry.expectedFont, `${label}: ${c.id} text is not reduced`);
              assert.ok(c.height >= geometry.controlHeight - 0.5, `${label}: ${c.id} is ${c.height}px, below the ${geometry.controlHeight}px control height`);
            }
            if (width >= 640) {
              assert.deepEqual(
                [geometry.panel.padding, geometry.panel.border, geometry.panel.background],
                [geometry.card.padding, geometry.card.border, geometry.card.background],
                `${label}: the panel keeps its card padding, border and fill`);
              for (const c of geometry.controls.slice(0, 2)) assert.deepEqual(c.paddingX, geometry.input.paddingX, `${label}: ${c.id} keeps its padding`);
            }
            // Diagnostic only: a native date can show in full below this width.
            const intrinsic = await control('report-filter-from').evaluate(el => {
              const clone = el.cloneNode();
              clone.style.cssText = 'position:absolute;visibility:hidden;width:max-content';
              document.body.append(clone);
              const width = clone.getBoundingClientRect().width;
              clone.remove();
              return width;
            });
            const fromBox = geometry.controls[0];
            console.log(`${label}: From ${(fromBox.right - fromBox.left).toFixed(2)}px rendered, ${intrinsic.toFixed(2)}px max-content (diagnostic)`);
            if (shots) {
              const name = `report-filter-reflow-${tab === 'Time' ? 'time' : 'detailed'}-${width}x${height}-${theme}${fontSize ? '-text32' : ''}`;
              await page.screenshot({ animations: 'disabled', path: join(shots, `${name}.png`) });
              if (width < 640 && fontSize) await page.locator('.card.mb-6').first().screenshot({ animations: 'disabled', path: join(shots, `${name}-panel.png`) });
            }
          }
        }
      }
    }
    await page.getByRole('button', { name: 'Time', exact: true }).click();

    // Keyboard editing at the narrowest enlarged size; the request and exports follow.
    await display(320, 900, 'dark', '32px');
    await settled();
    const [year, month, day] = from.split('-');
    await after('/api/report_time', args => args.from === '2000-01-01', () => control('report-filter-from').fill('2000-01-01'));
    await settled();
    await after('/api/report_time', args => args.from === from, async () => {
      await page.locator('label[for="report-filter-from"]').click();
      await page.keyboard.type(`${month}${day}${year}`);
    });
    await settled();
    await expect(control('report-filter-from')).toHaveValue(from);
    assert.deepEqual(await exportHrefs(), expectedHrefs(`from=${from}&to=${to}`));

    // A report failure keeps its alert and retry inside the narrow viewport.
    await page.route('**/api/report_time*', route => route.fulfill({ status: 500, body: 'fixture failure' }));
    await control('report-filter-group').selectOption('client');
    const retry = page.getByRole('button', { name: 'Retry report', exact: true });
    await expect(retry).toBeVisible();
    const retryBox = await retry.boundingBox();
    assert.ok(retryBox.x >= 0 && retryBox.x + retryBox.width <= 320, 'Retry stays inside the viewport');
    await page.unroute('**/api/report_time*');
    await after('/api/report_time', args => args.group_by === 'client', () => retry.click());
    await settled();

    // Long names, chosen and opened, at desktop and at the narrowest enlarged size.
    for (const [width, fontSize] of [[1440, ''], [320, '32px']]) {
      await display(width, 900, 'dark', fontSize);
      const where = `${width}px${fontSize ? ' text 32px' : ''}`;
      await after('/api/report_time', args => args.client_id === fixture.client,
        () => control('report-filter-client').selectOption(fixture.client));
      await settled();
      await after('/api/report_time', args => args.tag_id === fixture.tag,
        () => control('report-tag').selectOption(fixture.tag));
      await settled();
      for (const [id, name] of [['report-filter-client', fixture.clientName], ['report-tag', fixture.tagName]]) {
        assert.equal(await control(id).evaluate(el => el.selectedOptions[0].textContent.trim()), name, `${where}: ${id} keeps the full chosen name`);
        await control(id).focus();
        await page.keyboard.press('Alt+ArrowDown');
        assert.ok(await pageOverflow() <= 0, `${where}: opening ${id} does not widen the page`);
        await page.keyboard.press('Escape');
      }
      assert.ok(await pageOverflow() <= 0, `${where}: long chosen names do not widen the page`);
      if (shots) await page.locator('.card.mb-6').first().screenshot({ animations: 'disabled', path: join(shots, `report-filter-reflow-long-names-${width}${fontSize ? '-text32' : ''}.png`) });
      await after('/api/report_time', args => args.client_id === null && args.tag_id === null, async () => {
        await control('report-tag').selectOption('');
        await control('report-filter-client').selectOption('');
      });
      await settled();
    }

    // Behaviour at desktop width.
    await display(1440, 900, 'dark', '');
    for (const group of ['task', 'person', 'project']) {
      await after('/api/report_time', args => args.group_by === group, () => control('report-filter-group').selectOption(group));
      await settled();
    }
    // The client's own project list arrives separately from the report; pick from it.
    const client = sql(`SELECT c.id FROM clients c WHERE c.org_id='${orgId}' AND c.active
      AND EXISTS (SELECT 1 FROM projects p WHERE p.client_id=c.id AND p.active) ORDER BY c.id LIMIT 1`);
    const [projects] = await Promise.all([
      responseFor('/api/list_projects', args => args.client_id === client),
      responseFor('/api/report_time', args => args.client_id === client),
      control('report-filter-client').selectOption(client),
    ]);
    const listed = await projects.json();
    assert.ok(Array.isArray(listed) && listed.length > 0, `projects for the chosen client: ${JSON.stringify(listed)}`);
    const projectId = listed[0].id;
    await expect(control('report-filter-project').locator(`option[value="${projectId}"]`)).toBeAttached();
    await settled();
    await after('/api/report_time', args => args.project_id === projectId, () => control('report-filter-project').selectOption(projectId));
    await settled();
    assert.deepEqual(await exportHrefs(), expectedHrefs(`from=${from}&to=${to}&client_id=${client}&project_id=${projectId}`));
    await after('/api/report_time', args => args.client_id === null && args.project_id === null,
      () => control('report-filter-client').selectOption(''));
    await settled();
    await expect(control('report-filter-project')).toHaveValue('');
    assert.deepEqual(await exportHrefs(), expectedHrefs(`from=${from}&to=${to}`));

    // An empty range keeps both empty states.
    await after('/api/report_time', args => args.from === '2099-01-01', async () => {
      await control('report-filter-to').fill('2099-01-02');
      await control('report-filter-from').fill('2099-01-01');
    });
    await settled();
    await expect(page.getByText('No time tracked in this range.', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Detailed time', exact: true }).click();
    await settled();
    await expect(page.getByText('No entries in this range.', { exact: true })).toBeVisible();

    assert.deepEqual(errors, []);
    console.log(`PASS: legacy report filter labels (with Project tag), tab order, reflow at ${sizes.map(s => s.join('x')).join('/')} in both themes, base and 32px text, both tabs; keyboard date entry, retry, groupings, client reset, exports, empty states and long chosen names; Chromium ${browser.version()}`);
  } finally {
    try {
      await page.unrouteAll({ behavior: 'wait' });
    } finally {
      try {
        await browser.close();
      } finally {
        // A failed precondition never committed, so it must not delete anything.
        if (fixtureCommitted) sql(`BEGIN;
          DELETE FROM project_tag_links WHERE id='${fixture.link}';
          DELETE FROM project_tags WHERE id='${fixture.tag}';
          DELETE FROM clients WHERE id='${fixture.client}'; COMMIT;`);
      }
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
