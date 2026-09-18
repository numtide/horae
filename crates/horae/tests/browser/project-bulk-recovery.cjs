// All mutations are intercepted; this suite never changes project records.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port !== '8080');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  try {
    for (const scenario of ['success', 'pending', 'failure']) {
      const context = await browser.newContext();
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      let fixtures, saved = false, calls = 0, failRefresh = scenario === 'failure', release;
      try {
        await page.goto(`${base}/auth/login`);
        await page.getByRole('button', { name: 'Sign in as Admin' }).click();
        await page.waitForURL(`${base}/`);
        await page.route('**/api/**', async route => {
          const path = new URL(route.request().url()).pathname;
          if (path.startsWith('/api/set_projects_active')) {
            calls++;
            const body = route.request().postDataJSON();
            fixtures = fixtures.map(p => body.project_ids.includes(p.id) ? { ...p, active: body.active } : p);
            saved = true;
            return route.fulfill({ json: fixtures.filter(p => body.project_ids.includes(p.id)) });
          }
          if (/\/(create|update|set|delete|start|stop|import|submit|approve|reopen|cancel|retry)/.test(path)) {
            await route.abort();
            throw new Error(`Unexpected mutation: ${path}`);
          }
          if (path.startsWith('/api/list_projects')) {
            if (!fixtures) {
              const projects = await (await route.fetch()).json();
              assert.ok(projects.length);
              fixtures = [1, 2].map(i => ({ ...projects[0],
                id: `01950000-0000-7000-8000-${String(i).padStart(12, '0')}`,
                name: `Recovery project ${i}`, code: null, active: true }));
            }
            if (saved && failRefresh) return route.abort();
            if (saved && scenario === 'pending') await new Promise(resolve => { release = resolve; });
            return route.fulfill({ json: fixtures });
          }
          return route.continue();
        });
        await page.goto(`${base}/projects`);
        await expect(page.locator('.proj-row')).toHaveCount(2);
        await page.getByRole('checkbox', { name: 'Select all visible projects', exact: true }).click();
        const trigger = page.locator('#project-bulk-menu-trigger');
        const scope = page.locator('#project-scope-menu-trigger');
        await trigger.focus();
        await page.keyboard.press('Enter');
        await page.getByRole('menuitem', { name: 'Archive projects', exact: true }).focus();
        await page.keyboard.press('Enter');
        const dialog = page.getByRole('dialog', { name: 'Archive 2 projects?', exact: true });
        await dialog.getByRole('button', { name: 'Archive projects', exact: true }).focus();
        await page.keyboard.press('Enter');
        await expect(dialog).not.toBeVisible();
        await expect(page.getByRole('status')).toContainText('Archived 2 projects');
        await expect(scope).toBeFocused();
        await expect(trigger).toBeDisabled();
        if (scenario === 'pending') {
          await expect.poll(() => !!release).toBe(true);
          await expect(page.getByText('Loading projects…', { exact: true })).toBeVisible();
          await expect(page.getByRole('checkbox')).toHaveCount(0);
          await expect(page.locator('.proj-row')).toHaveCount(0);
          release();
        }
        if (scenario === 'failure') {
          const alert = page.getByRole('alert').filter({ hasText: 'Could not load projects' });
          await expect(alert).toBeVisible();
          await expect(page.getByRole('checkbox')).toHaveCount(0);
          failRefresh = false;
          await alert.getByRole('button', { name: 'Retry', exact: true }).click();
          await expect(alert).not.toBeVisible();
          await expect(scope).toBeFocused();
        }
        await expect(scope).toContainText('Active projects (0)');
        await expect(page.locator('.proj-row')).toHaveCount(0);
        await scope.click();
        await page.getByRole('menuitem', { name: /^Archived projects/ }).click();
        await expect(page.locator('.proj-row')).toHaveCount(2);
        assert.equal(calls, 1, 'Refreshing must never repeat the mutation');
        assert.deepEqual(errors, []);
        console.log(`PASS: bulk ${scenario} restores focus and recovers the list without stale actions`);
      } finally {
        if (release) release();
        await context.close();
      }
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
