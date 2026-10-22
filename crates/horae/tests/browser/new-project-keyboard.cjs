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
  // Focus scrolls an inner scroller to reveal its control, so geometry must be
  // read against the unscrolled scrollport, not only the viewport. Field cells,
  // row controls and dialog bodies are checked too, against their content boxes:
  // a control can spill into a gutter or padding without growing the scroll
  // width. Inputs are compared only by their boxes, so text scrolling inside a
  // field is never reported; a button whose own content is clipped is.
  const overflow = container => container.evaluate(node => {
    const visible = item => item.getClientRects().length && getComputedStyle(item).visibility !== 'hidden';
    const content = (item, rect) => {
      const style = getComputedStyle(item), left = rect.left + item.clientLeft + parseFloat(style.paddingLeft);
      return [left, rect.left + item.clientLeft + item.clientWidth - parseFloat(style.paddingRight)];
    };
    const name = item => `${item.tagName.toLowerCase()}${item.id ? `#${item.id}` : ''} ${item.getAttribute('aria-label') || item.textContent.trim().slice(0, 24)}`;
    const regions = [[node, ...content(node, node.getBoundingClientRect()), node.scrollLeft],
      ...[...node.querySelectorAll('.np-row > .min-w-0, .np-row-controls, .modal-body')].filter(visible)
        .map(cell => [cell, ...content(cell, cell.getBoundingClientRect()), 0])];
    const reported = new Set(), outside = [];
    for (const [region, left, right, shift] of regions) {
      for (const item of region.querySelectorAll('button, input:not([type="hidden"]), select, textarea, a[href], label, .form-group')) {
        const rect = item.getBoundingClientRect();
        if (outside.length >= 8 || reported.has(item) || !visible(item) || rect.width <= 1 || rect.height <= 1) continue;
        if (rect.left + shift < left - 1 || rect.right + shift > right + 1) {
          reported.add(item);
          outside.push(`${name(item)}: ${Math.round(rect.left + shift - left)}..${Math.round(rect.right + shift - left)} of ${Math.round(right - left)}${region === node ? '' : ` in .${region.classList[0]}`}`);
        } else if (item.tagName === 'BUTTON' && !item.matches('.truncate') && item.scrollWidth > item.clientWidth + 1) {
          reported.add(item);
          outside.push(`${name(item)}: content ${item.scrollWidth} clipped to ${item.clientWidth}`);
        }
      }
    }
    // Assignment-row text must keep room for itself: a role squeezed to zero width
    // can spill without any control moving. Names truncated on purpose are exempt.
    for (const cell of node.querySelectorAll('.np-assignment-row div, .np-assignment-row span')) {
      if (outside.length >= 8 || !visible(cell) || cell.closest('.truncate') || cell.children.length
        || !cell.textContent.trim() || getComputedStyle(cell).display === 'inline') continue;
      if (cell.clientWidth === 0 || cell.scrollWidth > cell.clientWidth + 1)
        outside.push(`squeezed ${cell.tagName.toLowerCase()}.${[...cell.classList].join('.')} "${cell.textContent.trim().slice(0, 32)}": content ${cell.scrollWidth} in ${cell.clientWidth}`);
    }
    // A wider scroll with no control outside comes from text, a legend or a
    // container: name the innermost few so the cause is identified, not guessed.
    if (node.scrollWidth > node.clientWidth + 1 && !outside.length) {
      const [left, right] = regions[0].slice(1, 3), shift = node.scrollLeft;
      const spills = item => {
        const rect = item.getBoundingClientRect();
        return rect.width > 1 && (rect.right + shift > right + 1 || rect.left + shift < left - 1)
          || (!['INPUT', 'TEXTAREA', 'SELECT'].includes(item.tagName) && getComputedStyle(item).overflowX === 'visible' && item.scrollWidth > item.clientWidth + 1);
      };
      for (const item of node.querySelectorAll('*')) {
        if (outside.length >= 3) break;
        if (!visible(item) || !spills(item) || [...item.children].some(child => visible(child) && spills(child))) continue;
        const rect = item.getBoundingClientRect();
        outside.push(`unexplained ${item.tagName.toLowerCase()}${item.id ? `#${item.id}` : ''}.${[...item.classList].join('.')} "${item.textContent.trim().slice(0, 32)}": ${Math.round(rect.left + shift - left)}..${Math.round(rect.right + shift - left)}, content ${item.scrollWidth} in ${item.clientWidth}`);
      }
      // Text can spill without any element box doing so; measure the text itself.
      const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT), range = document.createRange();
      for (let text = walker.nextNode(); text && outside.length < 6; text = walker.nextNode()) {
        if (!text.textContent.trim() || !visible(text.parentElement)) continue;
        range.selectNodeContents(text);
        const rect = range.getBoundingClientRect();
        if (rect.width > 1 && (rect.right + shift > right + 1 || rect.left + shift < left - 1))
          outside.push(`unexplained text in ${text.parentElement.tagName.toLowerCase()}.${[...text.parentElement.classList].join('.')} "${text.textContent.trim().slice(0, 32)}": ${Math.round(rect.left + shift - left)}..${Math.round(rect.right + shift - left)}`);
      }
    }
    return node.scrollWidth <= node.clientWidth + 1 && !outside.length ? ''
      : `scroll ${node.scrollWidth} > client ${node.clientWidth}; outside: ${JSON.stringify(outside)}`;
  });
  // Geometry is collected rather than thrown, so one overflow cannot hide the
  // rest of an enlarged-text run; it is asserted once those sections finish.
  const geometryProblems = [];
  const fitsViewport = (locator, label) => locator.evaluate(node => {
    const rect = node.getBoundingClientRect();
    return rect.left >= -1 && rect.right <= innerWidth + 1 && rect.top >= -1 && rect.bottom <= innerHeight + 1 ? ''
      : `${Math.round(rect.left)}..${Math.round(rect.right)} x ${Math.round(rect.top)}..${Math.round(rect.bottom)} in ${innerWidth}x${innerHeight}`;
  }).then(problem => { if (problem) geometryProblems.push(`${label}: ${problem}`); });
  // Keyboard-only date choice, also checking that the calendar and Today fit the viewport.
  const pickToday = async label => {
    const trigger = screen.locator('#np-start');
    await activate(trigger);
    const calendar = page.getByRole('dialog', { name: 'Choose Start date', exact: true });
    await expect(calendar).toBeVisible();
    await expect.poll(() => calendar.evaluate(node => node.contains(document.activeElement))).toBe(true);
    await fitsViewport(calendar, `${label} calendar`);
    const scrolls = await calendar.evaluate(node => node.scrollHeight <= node.clientHeight + 1 || ['auto', 'scroll'].includes(getComputedStyle(node).overflowY));
    if (!scrolls) geometryProblems.push(`${label} calendar: taller than its panel without scrolling`);
    const today = calendar.getByRole('button', { name: 'Today', exact: true });
    await today.focus();
    await expect(today).toBeFocused();
    await fitsViewport(today, `${label} calendar Today`);
    await page.keyboard.press('Enter');
    await expect(calendar).toBeHidden();
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
        const box = item.getBoundingClientRect();
        // A form control must show inside the form's visible scrollport, above the
        // pinned footer; a footer action inside the footer's own visible area.
        const port = (item.closest('.np-footer') || item.closest('.np-scroll'))?.getBoundingClientRect();
        const footer = document.querySelector('.np-footer')?.getBoundingClientRect();
        const ceiling = Math.max(port?.top ?? 0, 0);
        const floor = Math.min(port?.bottom ?? innerHeight, innerHeight, item.closest('.np-footer') ? innerHeight : footer?.top ?? innerHeight);
        return { control: `${item.tagName.toLowerCase()}${item.id ? `#${item.id}` : ''} ${item.getAttribute('aria-label') || item.textContent.trim().slice(0, 32)}`,
          top: Math.round(box.top), bottom: Math.round(box.bottom), left: Math.round(box.left), right: Math.round(box.right),
          ceiling: Math.round(ceiling), limit: Math.round(floor), height: innerHeight, width: innerWidth };
      });
      // Geometry joins the global report; focus, names and Tab order stay fatal.
      if (!(form ? geometry.top >= geometry.ceiling - 1 && geometry.bottom <= geometry.limit + 1
        : geometry.top >= -1 && geometry.bottom <= geometry.height + 1))
        geometryProblems.push(`${label}: focused control ${index} is vertically obscured: ${JSON.stringify(geometry)}`);
      if (!(geometry.left >= -1 && geometry.right <= geometry.width + 1))
        geometryProblems.push(`${label}: focused control ${index} is horizontally clipped: ${JSON.stringify(geometry)}`);
      visited.add(index);
      if (visited.size < count) await page.keyboard.press('Tab');
    }
    assert.equal(visited.size, count, `${label}: every visible enabled control is in the Tab order`);
    await root.locator('[data-keyboard-audit="0"]').focus();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    await expect(root.locator('[data-keyboard-audit="0"]')).toBeFocused();
    if (!await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)) geometryProblems.push(`${label}: document overflow`);
    if (!await root.evaluate(node => node.scrollWidth <= node.clientWidth + 1)) geometryProblems.push(`${label}: surface overflow`);
    const fit = await overflow(root.locator(form ? '.np-scroll' : '.modal'));
    if (fit) geometryProblems.push(`${label}: ${fit}`);
    console.log(`AUDITED: ${label}`);
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
    for (const [width, scale] of [[390, 1], [768, 1], [1440, 1], [1440, 2], [390, 2], [768, 2], [320, 2]]) {
      await page.setViewportSize({ width, height: 900 });
      // User text enlargement, applied only to the test document, not app styles.
      await page.evaluate(scale => { document.documentElement.style.fontSize = `${scale * 100}%`; }, scale);
      // 100% cases keep the original empty dates; every enlarged case has a date
      // set, so its clear control is measured, and opens the calendar once.
      if (scale === 2) {
        await pickToday(`${width}/${scale}`);
        await expect(screen.getByRole('button', { name: 'Clear Start date', exact: true })).toBeVisible();
      }
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
      console.log(`AUDITED: native Tab order, accessible names, reverse Tab and focus geometry across all billing/budget branches at ${width}px and ${scale * 100}% text; geometry acceptance follows the complete matrix`);
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
    // Audit only: repeating the access changes above would toggle its selection.
    for (const width of [320, 390, 768]) {
      await page.setViewportSize({ width, height: 900 });
      await page.evaluate(() => { document.documentElement.style.fontSize = '200%'; });
      for (const [trigger, name] of [
        [screen.getByRole('button', { name: '+ New client', exact: true }), 'New client'],
        [screen.getByRole('button', { name: /^Access for Development:/ }), 'Who can track to this task?'],
        [screen.getByRole('button', { name: 'Discard draft', exact: true }), 'Discard this draft?'],
      ]) {
        await saved();
        await readsFinished();
        await activate(trigger);
        const dialog = page.getByRole('dialog', { name, exact: true });
        await expect(dialog).toBeVisible();
        await audit(dialog, `${name} at ${width}px and 200% text`, false);
        await page.keyboard.press('Escape');
        await expect(dialog).toBeHidden();
        await expect(trigger).toBeFocused();
      }
    }
    // Short screens keep the pinned footer: the actual Tab order reaches every
    // control, and each focused control shows inside its own visible scrollport
    // (the form above the footer, or the footer's own area). 390x568 at 100% text
    // guards the normal-size layout.
    for (const [width, height, scale] of [[320, 568, 2], [390, 568, 2], [844, 390, 2], [390, 568, 1]]) {
      const size = `${width}x${height} at ${scale * 100}% text`;
      await page.setViewportSize({ width, height });
      await page.evaluate(scale => { document.documentElement.style.fontSize = `${scale * 100}%`; }, scale);
      await audit(screen, size);
      for (const control of [
        screen.getByLabel('Project name', { exact: true }),
        screen.locator('.np-footer').getByRole('button', { name: 'Save project', exact: true }),
        screen.locator('.np-footer').getByRole('button', { name: 'Cancel', exact: true }),
        screen.locator('.np-footer').getByRole('button', { name: 'Discard draft', exact: true }),
      ]) {
        await control.focus();
        await expect(control).toBeFocused();
        const shown = await control.evaluate(item => {
          const box = item.getBoundingClientRect(), port = item.closest('.np-footer, .np-scroll').getBoundingClientRect();
          const footer = document.querySelector('.np-footer').getBoundingClientRect();
          const floor = Math.min(port.bottom, innerHeight, item.closest('.np-footer') ? innerHeight : footer.top);
          return box.top >= Math.max(port.top, 0) - 1 && box.bottom <= floor + 1 && box.left >= -1 && box.right <= innerWidth + 1 ? ''
            : `${item.id || item.textContent.trim()} at ${Math.round(box.top)}..${Math.round(box.bottom)} outside ${Math.round(Math.max(port.top, 0))}..${Math.round(floor)}`;
        });
        if (shown) geometryProblems.push(`${size}: ${shown}`);
      }
    }
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    await page.setViewportSize({ width: 390, height: 900 });
    assert.deepEqual(geometryProblems, [], 'Enlarged-text matrix, calendar, dialogs and short screens fit their containers');
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
        if (type === 'Time & Materials') {
          await choose('np-budget-mode', 'Budget', 'Total project hours');
          await screen.locator('#np-budget-value').focus();
          await page.keyboard.type('30:30');
          for (const label of ['Budget resets every month', 'Include non-billable time in the budget']) {
            await screen.getByRole('checkbox', { name: label, exact: true }).focus();
            await page.keyboard.press('Space');
          }
          await saved();
          await readsFinished();
          await page.reload();
          await saved();
          await expect(screen.locator('#np-budget-value')).toHaveValue('30:30');
          for (const label of ['Budget resets every month', 'Include non-billable time in the budget']) {
            const checkbox = screen.getByRole('checkbox', { name: label, exact: true });
            await expect(checkbox).toBeChecked();
            if (width === 768) {
              await checkbox.focus();
              await page.keyboard.press('Space');
              await expect(checkbox).not.toBeChecked();
            }
          }
          // Hidden draft values must not become operational budget settings.
          if (width === 1440) await choose('np-budget-mode', 'Budget', 'No budget');
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
        if (type === 'Time & Materials') {
          const budget = JSON.parse(sql(`SELECT json_build_array(p.budget_minutes, s.monthly_reset, s.include_nonbillable, s.alert_enabled)
            FROM projects p JOIN project_settings s ON s.project_id = p.id WHERE p.id = '${id}'`));
          assert.deepEqual(budget, [width === 1440 ? null : 1830, width === 390, width === 390, false]);
          console.log(`PASS: budget amount and checkbox choices survive draft reload and finalize correctly at ${width}px`);
        }
        console.log(`PASS: keyboard-only ${type} creation at ${width}px took ${Math.round(elapsed)}ms and survives reopening`);
      }
    }
    assert.deepEqual(geometryProblems, [], 'Later audits fit their containers');
    assert.deepEqual(errors, []);
    console.log('PASS: modal traversal/Escape restores focus and configured email threshold is keyboard reachable without sending mail');
  } finally {
    // A fatal keyboard failure must not lose the geometry already collected.
    if (geometryProblems.length) console.error(`Collected geometry problems (${geometryProblems.length}):\n${geometryProblems.join('\n')}`);
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
