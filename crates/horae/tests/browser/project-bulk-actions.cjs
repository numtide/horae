// The full suite mutates only the runner's disposable database on port 8093.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const base = process.env.HORAE_TEST_URL;
const selectionOnly = process.argv.includes('--selection-only');
const fixturesOnly = process.argv.includes('--fixtures-only');
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port !== '8080');
if (!selectionOnly && !fixturesOnly) {
  assert.equal(target.port, '8093', 'Full suite requires the disposable runner');
  assert.ok(process.env.DATABASE_URL?.includes('?host=/'), 'Require the runner socket database');
}

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', error => {
    errors.push(error.message);
    console.error(`Browser error at ${page.url()}: ${error.stack}`);
  });
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    let role = 'admin', amount = 2, fail = false, hold = false, release;
    let calls = 0;
    let fixtures;
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname;
      if (path.startsWith('/api/set_projects_active')) {
        calls++;
        if (hold) await new Promise(resolve => { release = resolve; });
        if (fail) return route.abort();
        const body = route.request().postDataJSON();
        fixtures = fixtures.map(p => body.project_ids.includes(p.id) ? { ...p, active: body.active } : p);
        return route.fulfill({ json: fixtures.filter(p => body.project_ids.includes(p.id)) });
      }
      if (/\/(create|update|set|delete|start|stop|import|submit|approve|reopen|cancel|retry)/.test(path)) {
        throw new Error(`Unexpected mutation in fixture tests: ${path}`);
      }
      if (path.startsWith('/api/list_projects')) {
        if (!fixtures) {
          const projects = await (await route.fetch()).json();
          assert.ok(projects.length);
          fixtures = Array.from({ length: amount }, (_, i) => ({ ...projects[0],
            id: `01950000-0000-7000-8000-${String(i + 1).padStart(12, '0')}`,
            name: `Bulk project ${i + 1}`, code: null, active: true }));
        }
        return route.fulfill({ json: fixtures });
      }
      if (path.startsWith('/api/get_me')) {
        return route.fulfill({ json: { ...await (await route.fetch()).json(), org_role: role } });
      }
      return route.continue();
    });
    async function visit() {
      await page.goto(`${base}/projects`);
      // Zero rows also matches the loading state; wait for the resolved empty view
      // before the next navigation can cancel response bodies still being read.
      if (amount === 0) {
        await expect(page.getByRole('heading', { name: 'No projects yet', exact: true })).toBeVisible();
      }
      await expect(page.locator('.proj-row')).toHaveCount(amount);
    }
    const all = page.getByRole('checkbox', { name: 'Select all visible projects', exact: true });
    const bulkMenu = page.locator('#project-bulk-menu');
    const trigger = page.locator('#project-bulk-menu-trigger');
    await visit();
    await expect(all).toHaveAttribute('aria-checked', 'false');
    await expect(trigger).toBeDisabled();
    await trigger.click({ force: true });
    await trigger.dispatchEvent('keydown', { key: 'ArrowDown' });
    await expect(bulkMenu).not.toBeVisible();
    const first = page.getByRole('checkbox', { name: 'Select Bulk project 1', exact: true });
    await first.focus();
    await page.keyboard.press('Space');
    await expect(trigger).toBeEnabled();
    await expect(all).toHaveAttribute('aria-checked', 'mixed');
    await trigger.click();
    await expect(bulkMenu).toContainText('1 project selected');
    await page.keyboard.press('Escape');
    await all.click();
    await expect(all).toHaveAttribute('aria-checked', 'true');
    await all.click();
    await expect(first).toHaveAttribute('aria-checked', 'false');
    await expect(trigger).toBeDisabled();
    console.log('PASS: keyboard selection, mixed/select-all states and Actions count');

    await all.click();
    const search = page.getByRole('textbox', { name: 'Search by project or client' });
    await search.fill('Bulk project 1');
    await expect(first).toHaveAttribute('aria-checked', 'false');
    await all.click();
    await search.fill('');
    await expect(all).toHaveAttribute('aria-checked', 'false');
    await all.click();
    await page.getByRole('button', { name: /^Active projects/ }).click();
    await page.getByRole('menuitem', { name: /^Budgeted projects/ }).click();
    await expect(all).toHaveAttribute('aria-checked', 'false');
    await all.click();
    await page.getByRole('button', { name: /^All clients/ }).click();
    await page.getByRole('listbox').getByRole('button').filter({ hasNotText: 'All clients' }).first().click();
    await expect(all).toHaveAttribute('aria-checked', 'false');
    await visit();
    assert.equal(calls, 0);
    console.log('PASS: search, status and client changes clear selection');

    for (const theme of ['dark', 'light']) {
      await page.evaluate(theme => document.documentElement.setAttribute('data-theme', theme), theme);
      for (const width of [320, 768, 1440]) {
        await page.setViewportSize({ width, height: 700 });
        await all.focus();
        await page.keyboard.press('Space');
        await expect(all).toHaveAttribute('aria-checked', 'true');
        await page.keyboard.press('Space');
        await expect(all).toHaveAttribute('aria-checked', 'false');
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
        await expect(first.locator('.choice-box')).toHaveCSS('width', '18px');
        const selectionFits = await page.locator('.proj-head, .proj-row').evaluateAll(rows => rows.every(row => {
          const checkbox = row.querySelector('.choice-box').getBoundingClientRect();
          const cell = row.firstElementChild.getBoundingClientRect();
          const identity = row.children[1].getBoundingClientRect();
          const gap = parseFloat(getComputedStyle(row.parentElement).columnGap);
          return checkbox.left >= cell.left - 1 && checkbox.right <= cell.right + 1
            && identity.left - checkbox.right >= gap - 1;
        }));
        assert.equal(selectionFits, true, `Checkboxes fit their tracks without overlapping labels at ${width}px (${theme})`);
        await all.click();
        await trigger.focus();
        await page.keyboard.press('Enter');
        await expect(bulkMenu).toBeVisible();
        await page.keyboard.press('Escape');
        await expect(trigger).toBeFocused();
        await trigger.focus();
        await page.keyboard.press('Enter');
        await bulkMenu.getByRole('menuitem', { name: 'Archive projects', exact: true }).focus();
        await page.keyboard.press('Space');
        const confirmation = page.getByRole('dialog', { name: 'Archive 2 projects?', exact: true });
        await expect(confirmation).toBeVisible();
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
        await confirmation.getByRole('button', { name: 'Cancel', exact: true }).focus();
        await page.keyboard.press('Enter');
        await expect(confirmation).not.toBeVisible();
        await expect(trigger).toBeFocused();
        await all.click();
      }
    }
    console.log('PASS: keyboard selection/confirmation and no document overflow in both themes at three widths');
    await page.setViewportSize({ width: 1440, height: 900 });
    if (selectionOnly) {
      assert.deepEqual(errors, []);
      return;
    }

    async function confirmDialog(action = 'Archive') {
      await trigger.click();
      await bulkMenu.getByRole('menuitem', { name: `${action} projects`, exact: true }).click();
      const dialog = page.getByRole('dialog', { name: `${action} 2 projects?`, exact: true });
      await expect(dialog).toBeVisible();
      await expect(dialog).toContainText('Bulk project 1');
      await expect(dialog).toContainText('Bulk project 2');
      return dialog;
    }
    await all.click();
    let dialog = await confirmDialog();
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(trigger).toBeFocused();
    assert.equal(calls, 0);
    dialog = await confirmDialog();
    fail = true;
    await dialog.getByRole('button', { name: 'Archive projects', exact: true }).click();
    await expect(dialog.getByRole('alert')).toContainText('Could not');
    fail = false;
    hold = true;
    await dialog.getByRole('button', { name: 'Archive projects', exact: true }).click();
    await expect(dialog).toHaveAttribute('aria-busy', 'true');
    await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    await page.keyboard.press('Escape');
    await expect(dialog).toBeVisible();
    assert.equal(calls, 2);
    hold = false;
    release();
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole('status')).toContainText('Archived 2 projects');
    await expect(page.locator('.proj-row')).toHaveCount(0);
    await page.getByRole('button', { name: /^Active projects/ }).click();
    await page.getByRole('menuitem', { name: /^Archived projects/ }).click();
    await expect(all).toHaveAttribute('aria-checked', 'false');
    await all.click();
    dialog = await confirmDialog('Reactivate');
    await dialog.getByRole('button', { name: 'Reactivate projects', exact: true }).click();
    await expect(page.getByRole('status')).toContainText('Reactivated 2 projects');
    console.log('PASS: cancel/focus, failure retry, pending guard and archive/reactivate confirmation');

    amount = 101; fixtures = undefined;
    await visit();
    await all.click();
    await trigger.click();
    await expect(bulkMenu).toContainText('Select at most 100 projects');
    await expect(bulkMenu.getByRole('menuitem', { name: 'Archive projects', exact: true })).toBeDisabled();
    role = 'manager'; amount = 0; fixtures = undefined;
    await visit();
    await expect(page.getByRole('checkbox')).toHaveCount(0);
    await expect(trigger).toBeDisabled();
    role = 'member'; amount = 2; fixtures = undefined;
    await visit();
    await expect(page.getByRole('checkbox')).toHaveCount(0);
    await expect(trigger).toHaveCount(0);
    console.log('PASS: oversized selection cannot submit and members have no bulk controls');

    if (fixturesOnly) {
      assert.deepEqual(errors, []);
      return;
    }

    // Real requests, real sessions, and only the disposable runner database.
    await page.unroute('**/api/**');
    const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-XAt', '-v', 'ON_ERROR_STOP=1', '-c', query], { encoding: 'utf8' }).trim();
    const history = () => ['projects', 'tasks', 'project_tasks', 'assignments', 'time_entries', 'invoices', 'invoice_line_items']
      .map(table => sql(`SELECT coalesce(jsonb_agg(${table === 'projects' ? "to_jsonb(t) - 'active'" : 'to_jsonb(t)'} ORDER BY to_jsonb(t)), '[]') FROM ${table} t`));
    const originalHistory = history();
    let request;
    page.on('request', r => { if (r.url().includes('/api/set_projects_active')) request = r; });
    await page.goto(`${base}/projects`);
    await expect(page.locator('.proj-row')).toHaveCount(2);
    await all.click();
    await trigger.click();
    await bulkMenu.getByRole('menuitem', { name: 'Archive projects', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Archive projects', exact: true }).click();
    await expect(page.getByRole('status')).toContainText('Archived 2 projects');
    assert.deepEqual(history(), originalHistory, 'Archiving changes status only, not billing/history/assignments');
    assert.ok(request);
    const payload = request.postDataJSON();
    const url = request.url();
    const headers = { 'content-type': request.headers()['content-type'] };
    const adminId = sql("SELECT id FROM users WHERE org_role = 'admin' ORDER BY created_at LIMIT 1");
    assert.match(adminId, /^[0-9a-f-]{36}$/);
    const post = (context, body) => context.post(url, { headers, data: JSON.stringify(body) });
    try {
      const anonymous = await browser.newContext();
      assert.equal((await post(anonymous.request, payload)).status(), 401);
      await anonymous.close();
      for (const denied of ["org_role = 'member'", "org_role = 'admin', active = false"]) {
        sql(`UPDATE users SET ${denied} WHERE id = '${adminId}'`);
        const response = await post(page.request, { ...payload, active: true });
        assert.ok([401, 403].includes(response.status()), `Denied user got ${response.status()}`);
        assert.equal(sql('SELECT count(*) FROM projects WHERE active'), '0');
      }
      for (const allowed of ['manager', 'admin']) {
        sql(`UPDATE users SET org_role = '${allowed}', active = true WHERE id = '${adminId}'`);
        assert.equal((await post(page.request, { ...payload, active: true })).status(), 200);
        assert.equal(sql('SELECT count(*) FROM projects WHERE active'), '2');
        assert.equal((await post(page.request, payload)).status(), 200);
      }
      const foreign = '01950000-0000-7000-8000-900000000003';
      sql("INSERT INTO organizations (id, name) VALUES ('01950000-0000-7000-8000-900000000001', 'Other organization'); INSERT INTO clients (id, org_id, name, currency) VALUES ('01950000-0000-7000-8000-900000000002', '01950000-0000-7000-8000-900000000001', 'Other client', 'EUR'); INSERT INTO projects (id, org_id, client_id, name, currency, active) VALUES ('01950000-0000-7000-8000-900000000003', '01950000-0000-7000-8000-900000000001', '01950000-0000-7000-8000-900000000002', 'Foreign project', 'EUR', false)");
      const crossOrg = { ...payload, project_ids: [...payload.project_ids, foreign], active: true };
      assert.equal((await post(page.request, crossOrg)).status(), 404);
      assert.equal(sql('SELECT count(*) FROM projects WHERE active'), '0');
      assert.equal((await post(page.request, { ...payload, project_ids: [], active: true })).status(), 400);
      assert.equal((await post(page.request, { ...payload, project_ids: ['bad'], active: true })).status(), 400);
      assert.equal((await post(page.request, { ...payload, project_ids: Array(101).fill(payload.project_ids[0]), active: true })).status(), 400);
      assert.equal(sql('SELECT count(*) FROM projects WHERE active'), '0');
      const duplicated = await post(page.request, { ...payload, project_ids: [...payload.project_ids, payload.project_ids[0]], active: true });
      assert.equal(duplicated.status(), 200);
      assert.equal((await duplicated.json()).length, 2);
      assert.equal((await post(page.request, { ...payload, active: true })).status(), 200);
      await page.reload();
      await expect(page.locator('.proj-row')).toHaveCount(2);
    } finally {
      sql(`UPDATE users SET org_role = 'admin', active = true WHERE id = '${adminId}'`);
    }
    assert.deepEqual(errors, []);
    console.log('PASS: real atomic archive/reactivate, anonymous/member/inactive denial, manager/admin and safe retries');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
