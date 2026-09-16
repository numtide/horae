// Use an isolated, seeded dev-login instance, never the live imported workspace.
// HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/projects-design.cjs
// Read responses are intercepted for UI fixtures; no business mutation is allowed.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port !== '8080',
  'Use a separate loopback test instance, not the live app');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const mutations = [];
  let empty = false;
  let role = 'admin';
  let spendFails = false;
  let holdSpend = false;
  let releaseSpend;
  let fixtureProject;
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname;
      if (/\/(create|update|set|delete|start|stop|import|submit|approve|reopen|cancel|retry)/.test(path)) {
        mutations.push(path);
        return route.abort();
      }
      if (path.startsWith('/api/list_project_spend')) {
        if (holdSpend) await new Promise(resolve => { releaseSpend = resolve; });
        if (spendFails) return route.abort();
        return route.fulfill({ json: [{ project_id: '01950000-0000-7000-8000-000000000005', spent_minutes: 60, spent_cents: 5000 }] });
      }
      if (path.startsWith('/api/list_clients')) {
        const response = await route.fetch();
        const clients = await response.json();
        assert.ok(clients.length, 'The test instance must contain seeded clients');
        return route.fulfill({ response, json: [...clients, { ...clients[0],
          id: '01950000-0000-7000-8000-000000000999', name: 'No projects client', active: true }] });
      }
      if (path.startsWith('/api/list_projects')) {
        const response = await route.fetch();
        const projects = await response.json();
        assert.ok(projects.length, 'The test instance must contain seeded projects');
        fixtureProject = { ...projects[0], id: '01950000-0000-7000-8000-000000000005',
          name: 'Design budget project', code: null, active: true, budget_kind: 'amount',
          budget_amount_cents: 10000, currency: 'EUR' };
        return route.fulfill({ response, json: empty ? [] : [fixtureProject] });
      }
      if (path.startsWith('/api/get_me')) {
        const response = await route.fetch();
        return route.fulfill({ response, json: { ...await response.json(), org_role: role } });
      }
      return route.continue();
    });
    async function visit() {
      const ready = page.waitForResponse(r => r.url().includes('/api/list_projects') && r.status() === 200);
      await page.goto(`${base}/projects`);
      await (await ready).finished();
    }
    await visit();
    await expect(page.getByRole('button', { name: 'New project', exact: true })).toBeVisible();
    await expect(page.locator('.page-header').getByRole('link', { name: 'Import', exact: true })).toHaveAttribute('href', '/admin/importers');
    await expect(page.locator('.proj-head')).not.toContainText('Scheduled');
    await expect(page.locator('.proj-head')).not.toContainText('Delta');
    const progress = page.getByRole('progressbar', { name: 'Budget used for Design budget project' });
    await expect(progress).toHaveAttribute('value', '50');
    await expect(progress).toHaveAttribute('max', '100');
    await expect(page.locator('.proj-row [style]')).toHaveCount(0);
    console.log('PASS: header actions, supported metrics and labelled native budget progress');

    await page.getByRole('textbox', { name: 'Search by project or client' }).fill('no matching project');
    await expect(page.getByRole('heading', { name: 'No projects match your filters' })).toBeVisible();
    await page.getByRole('button', { name: 'Reset filters', exact: true }).click();
    await expect(page.locator('.proj-row')).toHaveCount(1);
    await expect(page.getByRole('textbox', { name: 'Search by project or client' })).toHaveValue('');
    await page.getByRole('button', { name: /^Active projects/ }).click();
    await page.getByRole('menuitem', { name: /^Archived projects/ }).click();
    await page.getByRole('button', { name: 'Reset filters', exact: true }).click();
    await expect(page.getByRole('button', { name: /^Active projects/ })).toBeVisible();
    await page.getByRole('button', { name: /^Filter by client/ }).click();
    await page.getByRole('listbox').getByRole('button', { name: 'No projects client', exact: true }).click();
    await page.getByRole('button', { name: 'Reset filters', exact: true }).click();
    await expect(page.getByRole('button', { name: /^Filter by client/ })).toBeVisible();
    await expect(page.locator('.proj-row')).toHaveCount(1);
    console.log('PASS: filtered empty state resets search, scope and client');

    for (const width of [320, 768, 1440]) {
      await page.setViewportSize({ width, height: 640 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
      const headerFits = await page.locator('.page-header').evaluate(header => {
        const bounds = header.getBoundingClientRect();
        return [...header.querySelectorAll('button, a, input')].every(el => {
          const box = el.getBoundingClientRect();
          return box.left >= bounds.left - 1 && box.right <= bounds.right + 1;
        });
      });
      assert.equal(headerFits, true, `Header controls fit at ${width}px`);
      const geometry = await page.locator('.proj-scroll').evaluate(scroll => {
        const head = [...scroll.querySelector('.proj-head').children].map(el => el.getBoundingClientRect());
        return [...scroll.querySelector('.proj-row').children].every((el, i) => {
          const box = el.getBoundingClientRect();
          return Math.abs(box.x - head[i].x) < 1 && Math.abs(box.width - head[i].width) < 1;
        });
      });
      assert.equal(geometry, true);
    }
    await page.locator('.proj-namelink').evaluate(el => { el.textContent = 'unbroken_project_name_'.repeat(20); });
    assert.equal(await page.locator('.proj-row').evaluate(row => {
      const name = row.firstElementChild.getBoundingClientRect();
      return [...row.firstElementChild.children].every(el => {
        const box = el.getBoundingClientRect();
        return box.right <= name.right + 1 && el.scrollWidth <= el.clientWidth + 1;
      });
    }), true, 'Long project names remain within the identity column');
    const scroller = page.getByRole('region', { name: 'Projects by client' });
    await scroller.focus();
    await expect(scroller).toBeFocused();
    const actions = page.locator('.proj-row').getByRole('button', { name: 'Actions' });
    await actions.focus();
    await page.keyboard.press('Enter');
    await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Edit Project', exact: true })).toBeVisible();
    await page.locator('.page-header').getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.getByRole('button', { name: 'Export', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Export projects' });
    await expect(dialog.getByRole('link', { name: 'Export projects' })).toHaveAttribute('href', '/api/projects/export/csv?scope=active');
    await dialog.getByRole('button', { name: 'Excel', exact: true }).click();
    await expect(dialog.getByRole('link', { name: 'Export projects' })).toHaveAttribute('href', '/api/projects/export/xlsx?scope=active');
    await page.keyboard.press('Escape');
    console.log('PASS: narrow/short layouts, keyboard row actions and export formats');

    holdSpend = true;
    await visit();
    await expect(page.getByRole('status')).toHaveText('Loading project spend…');
    await expect(page.locator('.proj-row [aria-label="Spent unavailable"]')).toHaveText('—');
    await expect(page.getByRole('progressbar')).toHaveCount(0);
    holdSpend = false;
    releaseSpend();
    await expect(progress).toHaveAttribute('value', '50');
    console.log('PASS: loading spend is unavailable until the read completes');

    spendFails = true;
    await visit();
    await expect(page.getByRole('alert')).toContainText('Could not load project spend');
    await expect(page.locator('.proj-row [aria-label="Spent unavailable"]')).toHaveText('—');
    await expect(page.locator('.proj-row [aria-label="Remaining unavailable"]')).toHaveText('—');
    await expect(page.getByRole('progressbar')).toHaveCount(0);
    spendFails = false;
    await page.getByRole('button', { name: 'Retry', exact: true }).click();
    await expect(progress).toHaveAttribute('value', '50');
    await expect(page.getByRole('alert')).toHaveCount(0);
    console.log('PASS: failed spend is unavailable; retry restores real amounts');

    empty = true;
    for (const [orgRole, canCreate, canImport] of [['admin', true, true], ['manager', true, false], ['member', false, false]]) {
      role = orgRole;
      await visit();
      const state = page.locator('.empty-state');
      await expect(state.getByRole('heading', { name: 'No projects yet', exact: true })).toBeVisible();
      await expect(state.getByRole('button', { name: 'New project', exact: true })).toHaveCount(canCreate ? 1 : 0);
      await expect(state.getByRole('link', { name: /Import from Harvest/ })).toHaveCount(canImport ? 1 : 0);
      if (canCreate) {
        await state.getByRole('button', { name: 'New project', exact: true }).click();
        await expect(page.getByRole('heading', { name: 'New Project', exact: true })).toBeVisible();
        await page.locator('.page-header').getByRole('button', { name: 'Cancel', exact: true }).click();
      }
      console.log(`PASS: empty-state guidance and action visibility for ${orgRole}`);
    }
    assert.deepEqual(mutations, []);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
