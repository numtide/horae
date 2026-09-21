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
  let budgetRows = [];
  let budgetFails = false;
  let holdBudget = false;
  let releaseBudget;
  let fixtureProject;
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname;
      if (/\/(create|update|set|delete|start|stop|import|submit|approve|reopen|cancel|retry|save|finalize|discard)/.test(path)) {
        mutations.push(path);
        return route.abort();
      }
      if (path.startsWith('/api/list_project_spend')) {
        if (holdSpend) await new Promise(resolve => { releaseSpend = resolve; });
        if (spendFails) return route.abort();
        return route.fulfill({ json: [{ project_id: '01950000-0000-7000-8000-000000000005', spent_minutes: 60, spent_cents: 5000 }] });
      }
      if (path.startsWith('/api/list_project_budget_progress')) {
        if (holdBudget) await new Promise(resolve => { releaseBudget = resolve; });
        if (budgetFails) return route.abort();
        return route.fulfill({ json: budgetRows });
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
      // Finish body decoding before replacing the document (Dioxus 0.7).
      await page.waitForLoadState('networkidle');
      const ready = page.waitForResponse(r => r.url().includes('/api/list_projects') && r.status() === 200);
      const [response] = await Promise.all([ready, page.goto(`${base}/projects`)]);
      await response.finished();
    }
    await visit();
    await expect(page.getByRole('link', { name: 'New project', exact: true })).toHaveAttribute('href', '/projects/new');
    await expect(page.locator('.page-header').getByRole('link', { name: 'Import', exact: true })).toHaveAttribute('href', '/admin/importers');
    await expect(page.locator('.proj-head')).not.toContainText('Scheduled');
    await expect(page.locator('.proj-head')).not.toContainText('Delta');
    await expect(page.locator('.proj-head')).toHaveCSS('color', 'rgb(106, 99, 83)');
    const progress = page.getByRole('progressbar', { name: 'Budget used for Design budget project' });
    await expect(progress).toHaveAttribute('value', '50');
    await expect(progress).toHaveAttribute('max', '100');
    await expect(page.locator('.proj-row [style]')).toHaveCount(0);
    console.log('PASS: header actions, supported metrics and labelled native budget progress');

    await expect(page.getByRole('heading', { name: 'Projects', exact: true })).toHaveCSS('font-size', '34px');
    await expect(page.locator('.proj-namelink')).toHaveCSS('font-size', '14px');
    assert.equal(await page.locator('.proj-row').evaluate(row => {
      const [name, type] = row.querySelector('.proj-namelink').parentElement.children;
      const a = name.getBoundingClientRect(), b = type.getBoundingClientRect();
      return Math.abs((a.top + a.bottom) / 2 - (b.top + b.bottom) / 2) < 1;
    }), true, 'Project name and type share a line');
    console.log('PASS: design heading, compact row typography and inline project type');

    await page.getByRole('textbox', { name: 'Search by project or client' }).fill('no matching project');
    await expect(page.getByRole('heading', { name: 'No projects match your filters' })).toBeVisible();
    await page.getByRole('button', { name: 'Reset filters', exact: true }).click();
    await expect(page.locator('.proj-row')).toHaveCount(1);
    await expect(page.getByRole('textbox', { name: 'Search by project or client' })).toHaveValue('');
    await page.getByRole('button', { name: /^Active projects/ }).click();
    await page.getByRole('menuitem', { name: /^Archived projects/ }).click();
    await page.getByRole('button', { name: 'Reset filters', exact: true }).click();
    await expect(page.getByRole('button', { name: /^Active projects/ })).toBeVisible();
    await page.getByRole('button', { name: /^All clients/ }).click();
    await page.getByRole('listbox').getByRole('button', { name: 'No projects client', exact: true }).click();
    await page.getByRole('button', { name: 'Reset filters', exact: true }).click();
    await expect(page.getByRole('button', { name: /^All clients/ })).toBeVisible();
    await expect(page.locator('.proj-row')).toHaveCount(1);
    console.log('PASS: filtered empty state resets search, scope and client');

    for (const width of [320, 768, 1440]) {
      await page.setViewportSize({ width, height: 640 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
      const headerFits = await page.locator('.page-header').evaluate(header => {
        const bounds = header.getBoundingClientRect();
        return [...header.querySelectorAll('button, a, input')].every(el => {
          // Closed popovers have no layout box and are not header controls.
          if (!el.checkVisibility()) return true;
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
      // Full currency codes and large/negative amounts must fit, not paint over
      // neighbouring columns. Only the browser DOM changes; no data is written.
      await page.locator('.proj-row').evaluate(row => {
        row.children[2].firstElementChild.textContent = 'USD 123,456,789.00';
        row.children[3].firstElementChild.textContent = 'USD 234,567,890.00';
        row.children[4].firstElementChild.textContent = '-USD 111,111,101.00';
      });
      assert.equal(await page.locator('.proj-row').evaluate(row => {
        return [...row.children].slice(2, 5).every(cell => {
          const bounds = cell.getBoundingClientRect();
          return [...cell.children].every(child => {
            const box = child.getBoundingClientRect();
            return box.left >= bounds.left - 1 && box.right <= bounds.right + 1;
          });
        });
      }), true, `Currency amounts stay inside their columns at ${width}px`);
    }
    await page.locator('.proj-namelink').evaluate(el => { el.textContent = 'unbroken_project_name_'.repeat(20); });
    assert.equal(await page.locator('.proj-row').evaluate(row => {
      const name = row.querySelector('.proj-namelink').parentElement.getBoundingClientRect();
      return [...row.querySelector('.proj-namelink').parentElement.children].every(el => {
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
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole('button', { name: 'Export', exact: true })).toBeFocused();
    console.log('PASS: narrow/short layouts, keyboard row actions and export formats');

    holdSpend = true;
    await visit();
    await expect(page.getByRole('status')).toHaveText('Loading project progress…');
    await expect(page.locator('.proj-row [aria-label="Spent unavailable"]')).toHaveText('—');
    await expect(page.getByRole('progressbar')).toHaveCount(0);
    holdSpend = false;
    releaseSpend();
    await expect(progress).toHaveAttribute('value', '50');
    console.log('PASS: loading spend is unavailable until the read completes');

    spendFails = true;
    await visit();
    await expect(page.getByRole('alert')).toContainText('Could not load project progress');
    await expect(page.locator('.proj-row [aria-label="Spent unavailable"]')).toHaveText('—');
    await expect(page.locator('.proj-row [aria-label="Remaining unavailable"]')).toHaveText('—');
    await expect(page.getByRole('progressbar')).toHaveCount(0);
    spendFails = false;
    await page.getByRole('button', { name: 'Retry', exact: true }).click();
    await expect(progress).toHaveAttribute('value', '50');
    await expect(page.getByRole('alert')).toHaveCount(0);
    console.log('PASS: failed spend is unavailable; retry restores real amounts');

    const configuredBudget = {
      project_id: '01950000-0000-7000-8000-000000000005', task_id: null, user_id: null,
      scope: 'project', label: null, kind: 'amount', currency: 'EUR',
      period_key: '2026-09', budget: 10000, consumed: 2500,
    };
    budgetRows = [configuredBudget];
    holdBudget = true;
    await visit();
    await expect(page.getByRole('status')).toHaveText('Loading project progress…');
    await expect(page.locator('.proj-row [aria-label="Budget unavailable"]')).toHaveText('—');
    await expect(page.getByRole('progressbar')).toHaveCount(0);
    await expect.poll(() => typeof releaseBudget).toBe('function');
    holdBudget = false;
    releaseBudget();
    await expect(progress).toHaveAttribute('value', '25');
    await expect(page.locator('.proj-row')).toContainText('Budget period: 2026-09');
    await expect(page.locator('.proj-row')).toContainText('Total tracked: 1h');
    await expect(page.locator('.proj-row')).toContainText('EUR 75.00');
    await expect(page.locator('[aria-label="Recurring budget"]')).toHaveCount(1);
    console.log('PASS: configured monthly consumption replaces lifetime spend only after loading');

    budgetRows = [
      { ...configuredBudget, scope: 'task', task_id: '01950000-0000-7000-8000-000000000010', label: 'Development', budget: 6000, consumed: 8000 },
      { ...configuredBudget, scope: 'task', task_id: '01950000-0000-7000-8000-000000000011', label: 'Review', budget: 4000, consumed: 0 },
    ];
    await visit();
    await expect(progress).toHaveAttribute('value', '80');
    const breakdown = page.locator('.proj-row details');
    await breakdown.locator('summary').focus();
    await page.keyboard.press('Enter');
    await expect(breakdown).toHaveAttribute('open', '');
    await expect(breakdown.locator('li').first()).toHaveText('Development: Budget EUR 60.00 · Spent EUR 80.00 · Remaining EUR -20.00');
    await expect(breakdown.locator('li').nth(1)).toHaveText('Review: Budget EUR 40.00 · Spent EUR 0.00 · Remaining EUR 40.00');
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);
      if (process.env.HORAE_TEST_SCREENSHOT_DIR)
        await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/projects-budget-${width}.png` });
    }
    await expect(page.locator('.proj-row [style]')).toHaveCount(0);
    console.log('PASS: keyboard-accessible scope breakdown exposes overruns without layout overflow');

    budgetFails = true;
    await visit();
    await expect(page.getByRole('alert')).toContainText('Could not load project progress');
    await expect(page.locator('.proj-row [aria-label="Budget unavailable"]')).toHaveText('—');
    await expect(page.getByRole('progressbar')).toHaveCount(0);
    budgetFails = false;
    await page.getByRole('button', { name: 'Retry', exact: true }).click();
    await expect(progress).toHaveAttribute('value', '80');
    console.log('PASS: failed configured budget never falls back to lifetime figures');
    budgetRows = [];

    empty = true;
    for (const [orgRole, canCreate, canImport] of [['admin', true, true], ['manager', true, false], ['member', false, false]]) {
      role = orgRole;
      await visit();
      const state = page.locator('.empty-state');
      await expect(state.getByRole('heading', { name: 'No projects yet', exact: true })).toBeVisible();
      await expect(state).toHaveCSS('border-radius', '16px');
      await expect(state).toHaveCSS('padding', '64px 24px');
      await expect(state).toHaveCSS('border-color', 'rgb(50, 46, 38)');
      await expect(state.locator('.empty-state-icon')).toHaveCSS('width', '52px');
      await expect(state.locator('.empty-state-icon')).toHaveCSS('height', '52px');
      await expect(state.locator('.empty-state-icon')).toHaveCSS('border-radius', '12px');
      await expect(state.locator('.empty-state-icon svg')).toHaveCSS('width', '22px');
      await expect(state.locator('.empty-state-icon svg')).toHaveCSS('height', '22px');
      await expect(state.locator('.empty-state-text')).toHaveCSS('max-width', '380px');
      await expect(state.locator('.empty-state-text')).toHaveCSS('color', 'rgb(143, 134, 118)');
      await expect(state.locator('.empty-state-text')).toHaveCSS('line-height', '21.7px');
      await expect(state.locator('.empty-state-text')).toHaveCSS('margin', '0px');
      await expect(state.locator('.empty-state-title')).toHaveCSS('margin', '0px');
      await expect(state.getByRole('link', { name: 'New project', exact: true })).toHaveCount(canCreate ? 1 : 0);
      await expect(state.getByRole('link', { name: /Import from Harvest/ })).toHaveCount(canImport ? 1 : 0);
      if (canImport) await expect(state.getByRole('link', { name: /Import from Harvest/ })).toHaveCSS('font-size', '14px');
      if (canImport && process.env.HORAE_TEST_SCREENSHOT_DIR)
        await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/projects-empty.png` });
      if (canCreate) {
        await expect(state.getByRole('link', { name: 'New project', exact: true })).toHaveCSS('padding', '12px 20px');
        await state.getByRole('link', { name: 'New project', exact: true }).click();
        await expect(page).toHaveURL(`${base}/projects/new`);
        await expect(page.getByRole('heading', { name: 'New project', exact: true })).toBeVisible();
        await page.getByRole('button', { name: /Back to Projects/ }).click();
        await expect(page).toHaveURL(`${base}/projects`);
      }
      console.log(`PASS: empty-state guidance and action visibility for ${orgRole}`);
    }
    await page.locator('html').evaluate(root => { root.dataset.theme = 'light'; });
    await expect(page.locator('.empty-state-text')).toHaveCSS('color', 'rgb(107, 100, 89)');
    await expect(page.locator('.empty-state')).toHaveCSS('background-color', 'rgb(248, 245, 238)');
    await page.locator('html').evaluate(root => { root.dataset.theme = 'dark'; });
    role = 'admin';
    await page.goto(`${base}/components`);
    for (const name of [/^Actions/, /^Filter by client/]) {
      const trigger = page.getByRole('button', { name });
      await expect(trigger).toHaveCSS('font-size', '12px');
      await expect(trigger).toHaveCSS('padding-left', '12px');
    }
    // Exercise the unmodified semantic defaults without depending on gallery copy.
    await page.locator('main').evaluate(main => {
      const state = document.createElement('div');
      state.id = 'default-empty-state';
      state.className = 'empty-state';
      const icon = document.createElement('span');
      icon.className = 'empty-state-icon';
      state.append(icon);
      main.append(state);
    });
    await expect(page.locator('#default-empty-state')).toHaveCSS('border-radius', '11px');
    await expect(page.locator('#default-empty-state')).toHaveCSS('padding', '64px 32px');
    await expect(page.locator('#default-empty-state .empty-state-icon')).toHaveCSS('width', '46px');
    await expect(page.locator('#default-empty-state .empty-state-icon')).toHaveCSS('border-radius', '50%');
    await expect(page.locator('.nav-item svg').first()).toHaveCSS('width', '15px');
    console.log('PASS: shared Menu, Combobox, empty-state and navigation-icon defaults are unchanged');
    assert.deepEqual(mutations, []);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
