const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
const database = new URL(process.env.DATABASE_URL);
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const pending = new Set(), errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) pending.add(request); });
  page.on('requestfinished', request => pending.delete(request));
  page.on('requestfailed', request => pending.delete(request));
  const readsFinished = () => expect.poll(() => pending.size).toBe(0);
  const screen = page.locator('.np-page');
  const saved = () => expect(screen.locator('header').getByRole('status')).toContainText('Draft saved at');
  const activate = async control => {
    await expect(control).toBeEnabled();
    await control.focus();
    await expect(control).toBeFocused();
    await page.keyboard.press('Enter');
  };
  const radio = async name => {
    const control = screen.getByRole('radio', { name });
    await control.focus();
    await page.keyboard.press('Space');
    await expect(control).toBeChecked();
    await expect(control).toBeFocused();
  };
  const choose = async (id, name, value) => {
    const trigger = screen.locator(`#${id}`);
    await activate(trigger);
    const panel = page.getByRole('dialog', { name: `Choose ${name}`, exact: true });
    await expect(panel).toBeVisible();
    await expect.poll(() => panel.evaluate(node => node.contains(document.activeElement))).toBe(true);
    await activate(panel.getByRole('option', { name: value, exact: true }));
    await expect(panel).toBeHidden();
    await expect(trigger).toBeFocused();
  };
  const audit = async (root, label, form = true) => {
    if (form) { await saved(); await readsFinished(); }
    const count = await root.evaluate(node => {
      node.querySelectorAll('[data-keyboard-audit]').forEach(item => item.removeAttribute('data-keyboard-audit'));
      const controls = [...node.querySelectorAll('button, input:not([type="hidden"]), select, textarea, a[href]')]
        .filter(item => !item.matches(':disabled') && item.tabIndex >= 0 && item.getClientRects().length
          && getComputedStyle(item).visibility !== 'hidden'
          && (item.type !== 'radio' || item.checked));
      controls.forEach((item, index) => { item.dataset.keyboardAudit = String(index); });
      return controls.length;
    });
    assert.ok(count > 0, label);
    const visited = new Set();
    await root.locator('[data-keyboard-audit="0"]').focus();
    for (let step = 0; step < count * 4 && visited.size < count; step++) {
      const active = await page.evaluate(() => document.hasFocus() && document.activeElement?.getAttribute('data-keyboard-audit'));
      if (active === null || active === false) {
        const missing = await root.locator('[data-keyboard-audit]').evaluateAll((items, seen) => items.filter(item => !seen.includes(item.dataset.keyboardAudit)).map(item => item.outerHTML), [...visited]);
        assert.fail(`${label}: focus left the surface; unreached controls: ${JSON.stringify(missing)}`);
      }
      // Native date segments retarget focus to their owning input; a CSS :focus
      // locator alone cannot follow every stop inside the browser's control.
      const current = root.locator(`[data-keyboard-audit="${active}"]`);
      await expect(current).toBeFocused();
      assert.ok(await current.evaluate(item => {
        if (item.type === 'date') return true; // The browser paints its focused date segment.
        const surface = item.closest('.np-type, .np-option, .input-group, .chip-input') || item;
        const style = getComputedStyle(surface);
        return item.matches(':focus-visible') && (style.boxShadow !== 'none'
          || (style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0));
      }), `${label}: control ${active} has a visible keyboard focus indicator`);
      const index = await current.getAttribute('data-keyboard-audit');
      assert.notEqual(index, null, `${label}: Tab escaped the active surface before reaching all controls`);
      await expect(current).toHaveAccessibleName(/\S/);
      const geometry = await current.evaluate(item => {
        const box = item.getBoundingClientRect(), footer = document.querySelector('.np-footer')?.getBoundingClientRect();
        return { top: box.top, bottom: box.bottom, left: box.left, right: box.right,
          limit: item.closest('.np-footer') ? innerHeight : footer?.top ?? innerHeight,
          height: innerHeight, width: innerWidth };
      });
      assert.ok(geometry.top >= -1 && geometry.bottom <= (form ? geometry.limit : geometry.height) + 1,
        `${label}: focused control ${index} is vertically obscured: ${JSON.stringify(geometry)}`);
      assert.ok(geometry.left >= -1 && geometry.right <= geometry.width + 1,
        `${label}: focused control ${index} is horizontally clipped`);
      visited.add(index);
      if (visited.size < count) await page.keyboard.press('Tab');
    }
    assert.equal(visited.size, count, `${label}: every visible enabled control is in the Tab order`);
    await root.locator('[data-keyboard-audit="0"]').focus();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    await expect(root.locator('[data-keyboard-audit="0"]')).toBeFocused();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, `${label}: no document overflow`);
    assert.equal(await root.evaluate(node => node.scrollWidth <= node.clientWidth + 1), true, `${label}: no surface overflow`);
    console.log(`PASS: ${label}`);
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await readsFinished();
    await page.goto(`${base}/projects/new`);
    await choose('np-client', 'Client', 'Acme Corp');
    await screen.getByLabel('Project name', { exact: true }).fill('Keyboard matrix project');
    await activate(screen.getByRole('button', { name: 'Development', exact: true }));
    await choose('np-add-person', 'teammate', 'Admin User');
    await choose('np-terms', 'Payment terms', 'Custom days');
    await screen.locator('#np-terms-days').fill('14');
    await activate(screen.getByRole('button', { name: 'Add a second tax', exact: true }));
    await screen.locator('#np-second-tax-name').fill('Additional tax');
    await screen.locator('#np-second-tax').fill('2');
    const hourly = [
      ['Person', 'No budget'], ['Task', 'Hours per task'], ['Project', 'Total project hours'],
      ['Person', 'Hours per person'], ['Task', 'Fees per task'], ['Project', 'Total project fees'],
    ];
    for (const [width, scale] of [[390, 1], [768, 1], [1440, 1], [1440, 2]]) {
      await page.setViewportSize({ width, height: 900 });
      // User text enlargement, applied only to the test document, not app styles.
      await page.evaluate(scale => { document.documentElement.style.fontSize = `${scale * 100}%`; }, scale);
      await radio(/^Time & Materials/);
      for (const [rate, budget] of hourly) {
        await radio(new RegExp(`^${rate} hourly rate`));
        await choose('np-budget-mode', 'Budget', budget);
        await expect(screen.locator('#np-project-rate')).toHaveCount(rate === 'Project' ? 1 : 0);
        await expect(screen.locator('input[id^="np-task-rate-"]')).toHaveCount(rate === 'Task' ? 1 : 0);
        await expect(screen.locator('input[id^="np-person-rate-"]')).toHaveCount(rate === 'Person' ? 1 : 0);
        await audit(screen, `${width}/${scale} hourly ${rate}/${budget}`);
      }
      await radio(/^Fixed Fee/);
      await audit(screen, `${width}/${scale} fixed/no budget`);
      for (const [fee, budget] of [['Single fee', 'Total project hours'], ['Milestones', 'Hours per task'], ['Monthly', 'Hours per person']]) {
        await radio(fee);
        await choose('np-budget-mode', 'Budget', budget);
        if (fee === 'Milestones' && await screen.locator('.np-milestone-row').count() === 0) {
          await activate(screen.getByRole('button', { name: 'Add milestone', exact: true }));
          await screen.getByLabel('Milestone name', { exact: true }).fill('Delivery');
        }
        await audit(screen, `${width}/${scale} fixed/${fee}`);
        if (fee === 'Monthly') {
          for (const day of ['1st of the month', '15th of the month', 'Last day of the month']) await choose('np-monthly-day', 'Monthly invoice day', day);
        }
      }
      await radio(/^Non-Billable/);
      await expect(screen.getByRole('region', { name: 'Invoice defaults', exact: true })).toHaveCount(0);
      await expect(screen.locator('input[name="np-rate-mode"], input[name="np-fee-mode"]')).toHaveCount(0);
      for (const budget of ['No budget', 'Total project hours', 'Hours per task', 'Hours per person']) {
        await choose('np-budget-mode', 'Budget', budget);
        await audit(screen, `${width}/${scale} non-billable/${budget}`);
      }
      await expect(screen.locator('#np-budget-alert')).toBeDisabled();
      console.log(`PASS: native Tab order, accessible names, reverse Tab and unclipped focus across all billing/budget branches at ${width}px and ${scale * 100}% text`);
    }
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    await page.setViewportSize({ width: 390, height: 900 });
    for (const [trigger, name] of [
      [screen.getByRole('button', { name: '+ New client', exact: true }), 'New client'],
      [screen.getByRole('button', { name: /^Access for Development:/ }), 'Who can track to this task?'],
      [screen.getByRole('button', { name: 'Discard draft', exact: true }), 'Discard this draft?'],
    ]) {
      await saved();
      await activate(trigger);
      const dialog = page.getByRole('dialog', { name, exact: true });
      await expect(dialog).toBeVisible();
      await audit(dialog, name, false);
      if (name === 'Who can track to this task?') {
        await dialog.getByRole('radio', { name: 'Only selected people', exact: true }).focus();
        await page.keyboard.press('Space');
        await audit(dialog, 'restricted task members', false);
        await dialog.getByRole('checkbox', { name: 'Admin User', exact: true }).focus();
        await page.keyboard.press('Space');
        await expect(dialog.getByRole('checkbox', { name: 'Admin User', exact: true })).toBeChecked();
        await activate(dialog.getByRole('button', { name: 'Apply access', exact: true }));
        await expect(dialog).toBeHidden();
        await expect(trigger).toBeFocused();
        await expect(trigger).toContainText('Restricted (1)');
        await activate(trigger);
        await dialog.getByRole('radio', { name: 'Everyone on the project', exact: true }).focus();
        await page.keyboard.press('Space');
      }
      await page.keyboard.press('Escape');
      await expect(dialog).toBeHidden();
      await expect(trigger).toBeFocused();
      if (name === 'Who can track to this task?') await expect(trigger).toContainText('Restricted (1)');
    }
    await activate(screen.getByRole('button', { name: '+ New client', exact: true }));
    const newClient = page.getByRole('dialog', { name: 'New client', exact: true });
    await newClient.getByLabel('Client name', { exact: true }).focus();
    await page.keyboard.type('Keyboard-created client');
    await activate(newClient.getByRole('button', { name: 'Create client', exact: true }));
    await expect(newClient).toBeHidden();
    await expect(screen.locator('#np-client')).toContainText('Keyboard-created client');
    await saved();
    await readsFinished();
    // Exercise configured-mail controls without running or claiming a delivery.
    await page.route('**/api/project_creation_options*', async route => {
      const response = await route.fetch();
      await route.fulfill({ response, json: { ...await response.json(), email_available: true } });
    });
    await page.reload();
    await saved();
    await screen.locator('#np-budget-alert').focus();
    await page.keyboard.press('Space');
    await expect(screen.locator('#np-budget-alert')).toBeChecked();
    await expect(screen.locator('#np-alert-threshold')).toBeEnabled();
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await audit(screen, `configured email threshold at ${width}`);
    }
    await screen.locator('#np-budget-alert').focus();
    await page.keyboard.press('Space');
    await expect(screen.locator('#np-alert-threshold')).toHaveCount(0);
    await saved();
    await readsFinished();
    await page.unroute('**/api/project_creation_options*');
    await activate(screen.getByRole('button', { name: 'Discard draft', exact: true }));
    await activate(page.getByRole('dialog', { name: 'Discard this draft?', exact: true }).getByRole('button', { name: 'Discard draft', exact: true }));
    await page.waitForURL(`${base}/projects`);
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      for (const type of ['Time & Materials', 'Fixed Fee', 'Non-Billable']) {
        await readsFinished();
        await page.goto(`${base}/projects`);
        const started = performance.now(), name = `Keyboard ${type} ${width}`;
        await activate(page.getByRole('link', { name: 'New project', exact: true }));
        await choose('np-client', 'Client', 'Acme Corp');
        await screen.getByLabel('Project name', { exact: true }).focus();
        await page.keyboard.type(name);
        await radio(new RegExp(`^${type}`));
        if (type === 'Fixed Fee') {
          await screen.locator('#np-fee-amount').focus();
          await page.keyboard.type('100');
        }
        await activate(screen.getByRole('button', { name: 'Save project', exact: true }));
        await page.waitForURL(/\/projects\/[0-9a-f-]{36}$/);
        const detail = page.getByRole('region', { name: 'Project details', exact: true });
        await expect(detail).toContainText(name);
        await expect(detail).toContainText('Acme Corp');
        const elapsed = performance.now() - started;
        assert.ok(elapsed < 120000, `Keyboard creation took ${elapsed}ms`);
        await readsFinished();
        await page.reload();
        await expect(detail).toContainText(name);
        await expect(detail).toContainText('Acme Corp');
        await readsFinished();
        const id = page.url().split('/').at(-1);
        assert.match(id, /^[0-9a-f-]{36}$/);
        assert.equal(sql(`SELECT project_type::text FROM projects WHERE id = '${id}'`), {
          'Time & Materials': 'time_and_materials', 'Fixed Fee': 'fixed_fee', 'Non-Billable': 'non_billable',
        }[type]);
        if (type === 'Fixed Fee') assert.equal(sql(`SELECT fee_amount_cents FROM project_settings WHERE project_id = '${id}' AND fee_mode = 'single'`), '10000');
        console.log(`PASS: keyboard-only ${type} creation at ${width}px took ${Math.round(elapsed)}ms and survives reopening`);
      }
    }
    assert.deepEqual(errors, []);
    console.log('PASS: modal traversal/Escape restores focus and configured email threshold is keyboard reachable without sending mail');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
