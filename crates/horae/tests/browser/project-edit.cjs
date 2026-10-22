// All writes belong to run-design-checks.sh's disposable database, never a preview.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
// The list editor URL; the router may end it with an empty query delimiter.
const atEditor = id => url => [`${base}/projects/${id}/edit`, `${base}/projects/${id}/edit?`].includes(url.href);
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const project = JSON.parse(sql(`SELECT row_to_json(p) FROM (
  SELECT p.id, p.name, p.code, p.currency, c.name AS client_name
  FROM projects p JOIN clients c ON c.id = p.client_id
  JOIN organizations o ON o.id = p.org_id
  WHERE o.name = 'Demo Org' AND p.code = 'ACME-01'
) p`));
assert.match(project.id, /^[0-9a-f-]{36}$/);
const before = sql(`SELECT json_build_array(
  (SELECT count(*) FROM projects),
  (SELECT count(*) FROM project_drafts),
  (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM time_entries t),
  (SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM invoices i),
  (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l)
)`);

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  const errors = [];
  let draftWrites = 0;
  page.on('request', request => {
    if (request.url().includes('/api/save_project_draft')) draftWrites++;
  });
  page.on('pageerror', error => errors.push(error.stack || error.message));
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`${base}/projects`);
      const row = page.locator('.proj-row').filter({ has: page.locator(`a[href="/projects/${project.id}"]`) });
      await row.getByRole('button', { name: /^Actions/ }).click();
      await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
      await expect(page).toHaveURL(atEditor(project.id));
      const editor = page.locator('.np-page');
      await expect(editor.getByRole('heading', { name: 'Edit project', exact: true })).toBeVisible();
      await expect(page.locator('#proj-name')).toHaveCount(0);
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(project.name);
      await expect(editor.locator('#np-code')).toHaveValue(project.code);
      await expect(editor.getByRole('button', { name: 'Client', exact: true })).toContainText(project.client_name);
      await expect(editor.getByRole('button', { name: 'Currency', exact: true })).toContainText(project.currency);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true);

      await editor.getByLabel('Project name', { exact: true }).fill('Cancelled project edit');
      await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
      const confirmation = page.getByRole('dialog');
      await expect(confirmation).toBeVisible();
      await confirmation.getByRole('button', { name: 'Discard changes', exact: true }).click();
      await expect(page).not.toHaveURL(atEditor(project.id));
      assert.equal(sql(`SELECT name FROM projects WHERE id = '${project.id}'`), project.name);

      await page.goto(`${base}/projects/${project.id}/edit`);
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(project.name);
      const name = `Shared project editor ${width}`;
      await editor.getByLabel('Project name', { exact: true }).fill(name);
      await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
      await expect(page).toHaveURL(`${base}/projects/${project.id}`);
      assert.equal(sql(`SELECT name FROM projects WHERE id = '${project.id}'`), name);
      project.name = name;
      await page.goto(`${base}/projects/${project.id}/edit`);
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(name);
      await page.reload();
      await expect(editor.getByLabel('Project name', { exact: true })).toHaveValue(name);
      console.log(`PASS: shared editor prefills, cancels, saves the same project and survives reload at ${width}px`);
    }
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
    // The focused element is fully on screen inside its own visible scrollport:
    // overlays against the viewport, footer actions inside the footer's area, and
    // form controls inside the form's scrollport above the pinned footer.
    const onScreen = () => page.evaluate(() => {
      const item = document.activeElement, rect = item.getBoundingClientRect();
      const overlay = item.closest('dialog, [role="dialog"]');
      const port = overlay ? null : (item.closest('.np-footer') || item.closest('.np-scroll'))?.getBoundingClientRect();
      const footer = document.querySelector('.np-footer')?.getBoundingClientRect();
      const top = Math.max(port?.top ?? 0, 0);
      const bottom = Math.min(port?.bottom ?? innerHeight, innerHeight,
        overlay || item.closest('.np-footer') ? innerHeight : footer?.top ?? innerHeight);
      return rect.left >= -1 && rect.right <= innerWidth + 1 && rect.top >= top - 1 && rect.bottom <= bottom + 1 ? ''
        : `${item.id || item.getAttribute('aria-label') || item.textContent.trim().slice(0, 24)} at ${Math.round(rect.left)}..${Math.round(rect.right)} x ${Math.round(rect.top)}..${Math.round(rect.bottom)} (visible ${Math.round(top)}..${Math.round(bottom)})`;
    });
    // Geometry is collected across sizes so one overflow cannot hide another.
    // 390x568 at 100% text guards the normal-size pinned layout.
    const problems = [];
    for (const [width, height, scale] of [[320, 900, 2], [390, 900, 2], [768, 900, 2], [320, 568, 2], [390, 568, 2], [844, 390, 2], [390, 568, 1]]) {
      const size = `${width}x${height} at ${scale * 100}% text`;
      await page.setViewportSize({ width, height });
      await page.goto(`${base}/projects/${project.id}/edit`);
      const reflow = page.locator('.np-page');
      const startsOn = sql(`SELECT coalesce(starts_on::text, '') FROM projects WHERE id = '${project.id}'`);
      await expect(reflow.getByLabel('Project name', { exact: true })).toHaveValue(project.name);
      // User text enlargement, applied only to the test document, not app styles.
      await page.evaluate(scale => { document.documentElement.style.fontSize = `${scale * 100}%`; }, scale);
      for (const selector of ['input[id^="np-task-rate-"]', 'input[id^="np-person-rate-"]', 'input[id^="np-cost-rate-"]'])
        await expect(reflow.locator(selector).first()).toBeVisible();
      const start = reflow.locator('#np-start');
      await start.click();
      const calendar = page.getByRole('dialog', { name: 'Choose Start date', exact: true });
      await expect(calendar).toBeVisible();
      await expect.poll(() => calendar.evaluate(node => node.contains(document.activeElement))).toBe(true);
      const panel = await calendar.evaluate(node => {
        const rect = node.getBoundingClientRect();
        const scrolls = node.scrollHeight <= node.clientHeight + 1 || ['auto', 'scroll'].includes(getComputedStyle(node).overflowY);
        return rect.left >= -1 && rect.right <= innerWidth + 1 && rect.top >= -1 && rect.bottom <= innerHeight + 1 && scrolls ? ''
          : `${Math.round(rect.left)}..${Math.round(rect.right)} x ${Math.round(rect.top)}..${Math.round(rect.bottom)}, scrolls ${scrolls}`;
      });
      if (panel) problems.push(`${size} calendar: ${panel}`);
      await calendar.getByRole('button', { name: 'Today', exact: true }).focus();
      const today = await onScreen();
      if (today) problems.push(`${size} calendar Today: ${today}`);
      await page.keyboard.press('Enter');
      await expect(calendar).toBeHidden();
      await expect(start).toBeFocused();
      await expect(reflow.getByRole('button', { name: 'Clear Start date', exact: true })).toBeVisible();
      // Ensure a dirty editor even if the saved planning date already is today.
      await reflow.getByLabel('Project name', { exact: true }).fill('Unsaved reflow check');
      const form = await overflow(reflow.locator('.np-scroll'));
      if (form) problems.push(`${size} form: ${form}`);
      if (!await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)) problems.push(`${size}: document overflow`);
      for (const action of await reflow.locator('.np-footer').getByRole('button').all()) {
        await action.focus();
        const fit = await onScreen();
        if (fit) problems.push(`${size} footer: ${fit}`);
      }
      if (height < 700) {
        // The actual Tab order from the name reaches the footer, each stop on screen.
        await reflow.getByLabel('Project name', { exact: true }).focus();
        let reached = false;
        for (let step = 0; step < 300 && !reached; step++) {
          const fit = await onScreen();
          if (fit && problems.length < 40) problems.push(`${size} Tab stop ${step}: ${fit}`);
          reached = await page.evaluate(() => !!document.activeElement.closest('.np-footer'));
          if (!reached) await page.keyboard.press('Tab');
        }
        if (!reached) problems.push(`${size}: Tab never reached the footer`);
      }

      const cancel = reflow.locator('.np-footer').getByRole('button', { name: 'Cancel', exact: true });
      await cancel.focus();
      await page.keyboard.press('Enter');
      const confirmation = page.getByRole('dialog', { name: 'Discard changes?', exact: true });
      await expect(confirmation).toBeVisible();
      const dialog = await overflow(confirmation.locator('.modal'));
      if (dialog) problems.push(`${size} discard dialog: ${dialog}`);
      for (const name of ['Discard changes', 'Keep editing']) {
        await confirmation.getByRole('button', { name, exact: true }).focus();
        const fit = await onScreen();
        if (fit) problems.push(`${size} discard dialog: ${fit}`);
      }
      await confirmation.getByRole('button', { name: 'Keep editing', exact: true }).focus();
      await page.keyboard.press('Enter');
      await expect(confirmation).toBeHidden();
      await expect(cancel).toBeFocused();
      await expect(reflow.getByRole('button', { name: 'Clear Start date', exact: true })).toBeVisible();
      await cancel.focus();
      await page.keyboard.press('Enter');
      await confirmation.getByRole('button', { name: 'Discard changes', exact: true }).focus();
      await page.keyboard.press('Enter');
      await expect(page).not.toHaveURL(atEditor(project.id));
      await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
      assert.equal(sql(`SELECT coalesce(starts_on::text, '') FROM projects WHERE id = '${project.id}'`), startsOn, 'Discarding leaves the saved planning date unchanged');
      assert.equal(sql(`SELECT name FROM projects WHERE id = '${project.id}'`), project.name, 'Discarding leaves the saved name unchanged');
      console.log(`DONE: legacy editor rows, planning date, footer and discard actions measured at ${size}`);
    }
    // The following cases were written for the desktop viewport the first loop left behind.
    await page.setViewportSize({ width: 1440, height: 900 });
    assert.equal(sql(`SELECT json_build_array(
      (SELECT count(*) FROM projects),
      (SELECT count(*) FROM project_drafts),
      (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM time_entries t),
      (SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM invoices i),
      (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l)
    )`), before, 'Editing must not create projects/drafts or rewrite historical time/invoices');
    assert.equal(draftWrites, 0, 'Existing-project editing must never autosave a creation draft');
    assert.deepEqual(problems, [], 'Legacy editor must reflow at enlarged text, including short screens');

    await page.goto(`${base}/projects/new`);
    const editor = page.locator('.np-page');
    await editor.locator('#np-client').click();
    await page.getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await editor.getByLabel('Project name', { exact: true }).fill('Configured edit fixture');
    await editor.locator('#np-code').fill('EDIT-CONFIG');
    await editor.locator('#np-notes').fill('Private context before editing');
    await editor.getByRole('radio', { name: /^Project hourly rate/ }).check();
    await editor.locator('#np-project-rate').fill('0');
    await editor.locator('#np-budget-mode').click();
    await page.getByRole('option', { name: 'Total project hours', exact: true }).click();
    await editor.locator('#np-budget-value').fill('10:30');
    await editor.getByRole('button', { name: 'Development', exact: true }).click();
    await editor.getByRole('button', { name: 'Add everyone', exact: true }).click();
    await expect(editor.getByRole('button', { name: 'Add everyone', exact: true })).toBeEnabled();
    await editor.locator('#np-po-number').fill('PO-BEFORE');
    await expect(editor.locator('header').getByRole('status')).toContainText('Draft saved at');
    await editor.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}$/);
    const configuredId = new URL(page.url()).pathname.split('/').pop();
    const draftWritesAfterCreate = draftWrites;
    const configuredBefore = sql(`SELECT json_build_array(
      (SELECT count(*) FROM projects),
      (SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM project_drafts d),
      (SELECT jsonb_agg(to_jsonb(t) ORDER BY task_id) FROM project_tasks t WHERE project_id = '${configuredId}'),
      (SELECT jsonb_agg(jsonb_build_array(id, user_id, created_at) ORDER BY id) FROM assignments WHERE project_id = '${configuredId}')
    )`);
    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await expect(editor.locator('#np-code')).toHaveValue('EDIT-CONFIG');
    await expect(editor.locator('#np-notes')).toHaveValue('Private context before editing');
    await expect(editor.getByRole('radio', { name: /^Project hourly rate/ })).toBeChecked();
    await expect(editor.locator('#np-project-rate')).toHaveValue('0.00');
    await expect(editor.locator('#np-budget-value')).toHaveValue('10:30');
    await expect(editor.locator('#np-po-number')).toHaveValue('PO-BEFORE');
    await expect(editor.getByRole('button', { name: 'Discard draft', exact: true })).toHaveCount(0);
    await editor.locator('#np-code').fill('EDIT-AFTER');
    await editor.locator('#np-project-rate').fill('37.25');
    await editor.locator('#np-budget-value').fill('20:45');
    await editor.locator('#np-notes').fill('Private context after editing');
    await editor.locator('#np-po-number').fill('PO-AFTER');
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${configuredId}`);
    assert.equal(sql(`SELECT code || ':' || rate_cents || ':' || budget_minutes FROM projects WHERE id = '${configuredId}'`), 'EDIT-AFTER:3725:1245');
    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-project-rate')).toHaveValue('37.25');
    await expect(editor.locator('#np-notes')).toHaveValue('Private context after editing');
    await expect(editor.locator('#np-po-number')).toHaveValue('PO-AFTER');
    const revision = sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`);
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${configuredId}`);
    assert.equal(sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`), revision, 'Unchanged save must remain a no-op');
    assert.equal(sql(`SELECT json_build_array(
      (SELECT count(*) FROM projects),
      (SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM project_drafts d),
      (SELECT jsonb_agg(to_jsonb(t) ORDER BY task_id) FROM project_tasks t WHERE project_id = '${configuredId}'),
      (SELECT jsonb_agg(jsonb_build_array(id, user_id, created_at) ORDER BY id) FROM assignments WHERE project_id = '${configuredId}')
    )`), configuredBefore, 'Configured editing preserves project, assignment and task identities and creation drafts');
    assert.equal(draftWrites, draftWritesAfterCreate);
    console.log('PASS: configured editor prefills exact zero, budgets, private notes and invoice defaults; persists changes and preserves identity on no-op');
    await page.goto(`${base}/projects`);
    const configuredRow = page.locator('.proj-row').filter({ has: page.locator(`a[href="/projects/${configuredId}"]`) });
    await configuredRow.getByRole('button', { name: /^Actions/ }).click();
    await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.locator('#np-name').fill('Keep this unsaved edit');
    const dismiss = async action => {
      const dialog = page.waitForEvent('dialog', { timeout: 5000 });
      const navigation = action();
      await (await dialog).dismiss();
      await navigation;
      await expect(page).toHaveURL(atEditor(configuredId));
      await expect(editor.locator('#np-name')).toHaveValue('Keep this unsaved edit');
    };
    await dismiss(() => page.locator('a.nav-item[href="/clients"]').click());
    await dismiss(() => page.evaluate(() => history.back()));
    await dismiss(() => page.evaluate(() => location.reload()));
    await dismiss(() => page.evaluate(() => history.replaceState(history.state, '', '/clients')));
    page.once('dialog', dialog => dialog.accept());
    await page.evaluate(() => history.back());
    await expect(page).toHaveURL(`${base}/projects`);
    await page.evaluate(() => history.forward());
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.evaluate(() => history.back());
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.locator('#np-name').fill('Keep this unsaved edit');
    await dismiss(() => page.evaluate(() => history.forward()));
    await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    assert.equal(sql(`SELECT name FROM projects WHERE id = '${configuredId}'`), 'Configured edit fixture');
    console.log('PASS: sidebar, back, forward, reload and replacement navigation preserve dirty edits until explicitly discarded');

    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-name')).toHaveValue('Configured edit fixture');
    await editor.locator('#np-name').fill('Committed with a lost acknowledgement');
    const requests = [];
    await page.route('**/api/save_project_editor*', async route => {
      requests.push(route.request().postData());
      await route.fetch();
      await route.abort('failed');
    });
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(editor.getByRole('alert')).toBeVisible();
    assert.equal(sql(`SELECT name FROM projects WHERE id = '${configuredId}'`), 'Committed with a lost acknowledgement');
    const savedRevision = sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`);
    await expect(editor.locator('#np-name')).toBeDisabled();
    await expect(editor.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    const blockedNavigation = page.waitForEvent('dialog');
    const sidebarClick = page.locator('a.nav-item[href="/clients"]').click();
    const pendingDialog = await blockedNavigation;
    assert.equal(pendingDialog.type(), 'alert');
    await pendingDialog.accept();
    await sidebarClick;
    await expect(page).toHaveURL(atEditor(configuredId));
    await page.unroute('**/api/save_project_editor*');
    await page.route('**/api/save_project_editor*', route => {
      requests.push(route.request().postData());
      return route.continue();
    });
    await editor.getByRole('button', { name: 'Retry request', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(`${base}/projects/${configuredId}`);
    assert.equal(requests.length, 2);
    assert.equal(requests[0], requests[1], 'Retry reuses the exact operation and snapshot');
    assert.equal(sql(`SELECT edit_revision FROM projects WHERE id = '${configuredId}'`), savedRevision);
    await page.unroute('**/api/save_project_editor*');
    console.log('PASS: lost save acknowledgement blocks navigation and retries the identical committed request without another update');

    await page.goto(`${base}/projects/${configuredId}/edit`);
    await expect(editor.locator('#np-name')).toHaveValue('Committed with a lost acknowledgement');
    await editor.locator('#np-name').fill('Local conflicting changes');
    const otherTab = await page.context().newPage();
    await otherTab.goto(`${base}/projects/${configuredId}/edit`);
    await otherTab.locator('#np-name').fill('Saved from another tab');
    await otherTab.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(otherTab).toHaveURL(`${base}/projects/${configuredId}`);
    await otherTab.close();
    await editor.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(editor.getByRole('alert')).toBeVisible();
    await expect(editor.locator('#np-name')).toHaveValue('Local conflicting changes');
    await page.setViewportSize({ width: 390, height: 900 });
    await page.evaluate(() => { document.documentElement.style.fontSize = '200%'; });
    const reloadTrigger = editor.getByRole('button', { name: 'Reload project…', exact: true });
    const reloadDialog = page.getByRole('dialog', { name: 'Reload project?', exact: true });
    await reloadTrigger.focus();
    await page.keyboard.press('Enter');
    await expect(reloadDialog).toBeVisible();
    const reloadOverflow = await overflow(reloadDialog.locator('.modal'));
    await reloadDialog.getByRole('button', { name: 'Keep editing', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(reloadDialog).toBeHidden();
    await expect(reloadTrigger).toBeFocused();
    await expect(editor.locator('#np-name')).toHaveValue('Local conflicting changes');
    await reloadTrigger.focus();
    await page.keyboard.press('Enter');
    await reloadDialog.getByRole('button', { name: 'Reload project', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(editor.locator('#np-name')).toHaveValue('Saved from another tab');
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    await page.setViewportSize({ width: 1440, height: 900 });
    assert.equal(reloadOverflow, '', 'Reload confirmation actions fit at 390px and 200% text');
    assert.equal(draftWrites, draftWritesAfterCreate);
    console.log('PASS: concurrent edits reject a stale save, retain local input and require confirmation before reloading');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
