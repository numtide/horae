// Row and bulk status actions on Projects keep the search, client filter and
// selection, and keep the list on screen while it reloads. Every mutation is
// fulfilled by the route fixture, so the runner's database is only read.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const expect = baseExpect.configure({ timeout: 15000 });
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port !== '8080');

const id = n => `01950000-0000-7000-8000-${String(n).padStart(12, '0')}`;
const acme = { id: id(901), name: 'Acme', active: true };
const beta = { id: id(902), name: 'Beta Corp', active: true };
const rows = [
  [1, 'Alpha one', acme], [2, 'Alpha two', acme], [3, 'Alpha three', acme],
  [4, 'Gamma', acme], [5, 'Alpha four', beta],
];

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);

    let fixtures, hold = null, failOverview = false, auxReads = 0;
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname;
      if (/^\/api\/(list_project_tags|list_project_spend|list_project_budget_progress)/.test(path)) auxReads++;
      if (/^\/api\/set_projects?_active/.test(path)) {
        const body = route.request().postDataJSON();
        const ids = body.project_ids || [body.project_id];
        fixtures = fixtures.map(p => ids.includes(p.id) ? { ...p, active: body.active } : p);
        const changed = fixtures.filter(p => ids.includes(p.id));
        return route.fulfill({ json: body.project_ids ? changed : changed[0] });
      }
      if (/\/(create|update|set|delete|start|stop|import|submit|approve|reopen|cancel|retry)/.test(path)) {
        throw new Error(`Unexpected mutation: ${path}`);
      }
      if (path.startsWith('/api/get_project_overview')) {
        if (hold) await hold.promise;
        if (failOverview) return route.abort();
        const overview = await (await route.fetch()).json();
        const template = overview.projects[0];
        fixtures ??= rows.map(([n, name, client]) => ({
          ...template.project, id: id(n), name, code: null, client_id: client.id, active: true,
          budget_kind: 'none', budget_amount_cents: null, budget_minutes: null,
        }));
        return route.fulfill({ json: {
          ...overview, canonical_permissions: false, can_create: true, can_change_legacy_status: true,
          projects: fixtures.map(project => ({
            ...template, project, can_edit: true,
            client: project.client_id === acme.id ? acme : beta,
          })),
        } });
      }
      return route.continue();
    });

    await page.goto(`${base}/projects`);
    await expect(page.locator('.proj-row')).toHaveCount(rows.length);
    const cdp = await page.context().newCDPSession(page);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 });
    // Count list remounts (the search box leaving the DOM) and loading swaps.
    await page.evaluate(() => {
      window.__metrics = { remounts: 0, loading: 0 };
      new MutationObserver(records => {
        for (const record of records) {
          for (const node of record.removedNodes) {
            if (node.nodeType === 1 && (node.matches('.proj-search-input') || node.querySelector('.proj-search-input'))) {
              window.__metrics.remounts++;
            }
          }
          for (const node of record.addedNodes) {
            if (node.textContent?.includes('Loading projects')) window.__metrics.loading++;
          }
        }
      }).observe(document.body, { childList: true, subtree: true });
    });

    const search = page.getByRole('textbox', { name: 'Search by project or client' });
    const clientTrigger = page.locator('button[aria-haspopup="listbox"]');
    const box = name => page.getByRole('checkbox', { name: `Select ${name}`, exact: true });
    const setChecked = async (name, on) => {
      if ((await box(name).getAttribute('aria-checked')) !== String(on)) await box(name).click();
    };
    await search.fill('alpha');
    await clientTrigger.click();
    await page.locator('.combobox-pop .menu-item', { hasText: /^Acme$/ }).click();
    await expect(page.locator('.proj-row')).toHaveCount(3);
    for (const name of ['Alpha one', 'Alpha two', 'Alpha three']) await setChecked(name, true);

    const results = {};
    async function measure(label, act, settled) {
      await page.evaluate(() => { window.__metrics.remounts = 0; window.__metrics.loading = 0; });
      const reads = auxReads;
      let release;
      hold = { promise: new Promise(resolve => { release = resolve; }) };
      const started = Date.now();
      await act();
      // While the reload is held, the list must still be on screen.
      await page.waitForTimeout(300);
      const during = await page.evaluate(() => ({
        rows: document.querySelectorAll('.proj-row').length,
        busy: document.querySelector('.proj-content')?.getAttribute('aria-busy') === 'true',
        loadingText: document.body.innerText.includes('Loading projects'),
        bulkDisabled: document.getElementById('project-bulk-menu-trigger')?.disabled,
      }));
      hold = null;
      release();
      await settled();
      const ms = Date.now() - started;
      // A remounted list starts its tag, spend and budget reads after it renders.
      await page.waitForTimeout(1000);
      const metrics = await page.evaluate(() => window.__metrics);
      results[label] = {
        ...metrics, ...during, auxReads: auxReads - reads, ms,
        search: await search.inputValue().catch(() => null),
        client: (await clientTrigger.textContent().catch(() => '')).replace('▾', '').trim(),
      };
    }

    const rowMenu = async (n, item) => {
      await page.locator(`#project-actions-${id(n)}-trigger`).click();
      await page.locator(`#project-actions-${id(n)}`).getByRole('menuitem', { name: item, exact: true }).click();
    };
    await measure('row archive', () => rowMenu(1, 'Archive'),
      () => expect(box('Alpha one')).toHaveCount(0));
    const afterRow = {
      two: await box('Alpha two').getAttribute('aria-checked').catch(() => null),
      three: await box('Alpha three').getAttribute('aria-checked').catch(() => null),
    };

    await setChecked('Alpha two', true);
    await setChecked('Alpha three', false);
    await measure('bulk archive', async () => {
      await page.locator('#project-bulk-menu-trigger').click();
      await page.locator('#project-bulk-menu').getByRole('menuitem', { name: 'Archive projects' }).click();
      await page.locator('#bulk-projects-dialog').getByRole('button', { name: 'Archive projects' }).click();
    }, () => expect(page.getByRole('status').filter({ hasText: 'Archived 1 project' })).toBeVisible());

    await setChecked('Alpha three', true);
    failOverview = true;
    await measure('failed reload', () => rowMenu(3, 'Archive'),
      () => expect(page.getByRole('alert').filter({ hasText: /Could not load projects|Project access is unavailable/ })).toBeVisible());
    const failedAlert = await page.getByRole('alert').filter({ hasText: /Could not load projects|Project access is unavailable/ }).textContent();
    // The failed reload hides the selected row, so no bulk action may apply to it.
    const bulkAfterFailure = await page.locator('#project-bulk-menu-trigger').isDisabled().catch(() => null);
    failOverview = false;
    await page.getByRole('button', { name: /^Retry( access)?$/ }).click();
    await expect(page.getByText(/No projects match your filters|Visible|Alpha four/).first()).toBeVisible();

    console.log(JSON.stringify({ results, afterRow, failedAlert, bulkAfterFailure }, null, 2));

    for (const [label, result] of Object.entries(results)) {
      assert.equal(result.remounts, 0, `${label}: the list remounted`);
      assert.equal(result.loading, 0, `${label}: "Loading projects…" replaced the list`);
      assert.equal(result.auxReads, 0, `${label}: tags, spend and budget were refetched`);
      assert.ok(result.rows > 0 && result.busy && !result.loadingText, `${label}: stale list not kept busy`);
      assert.equal(result.bulkDisabled, true, `${label}: bulk actions stay enabled over a held list`);
      assert.equal(result.search, 'alpha', `${label}: search lost`);
      assert.equal(result.client, 'Acme', `${label}: client filter lost`);
    }
    assert.deepEqual(afterRow, { two: 'true', three: 'true' }, 'selection lost after a row action');
    assert.match(failedAlert, /Could not load projects/);
    assert.equal(bulkAfterFailure, true, 'bulk actions stay enabled over rows a failed reload hid');
    await expect(page.getByText('No projects match your filters')).toBeVisible();
    assert.equal(await search.inputValue(), 'alpha');
    assert.deepEqual(errors, []);
    console.log('projects-keep-filters: ok');
  } finally {
    await browser.close();
  }
})().catch(error => {
  console.error(error);
  process.exit(1);
});
