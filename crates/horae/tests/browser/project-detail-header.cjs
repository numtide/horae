// Project Detail identity and archive/reactivate, against the runner's disposable database.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const expect = baseExpect.configure({ timeout: 15000 });
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id FROM users WHERE email='admin@example.com' AND active AND org_role='admin') u"));
const org = actor.org_id;
const id = n => `019f4200-0000-7000-8000-${String(n).padStart(12, '0')}`;
const project = id(1), uncoded = id(2), client = id(3), tag = id(4);
const active = () => sql(`SELECT active FROM projects WHERE id='${project}'`);
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${org}'`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const statusRequests = [];
  page.on('request', request => {
    if (new URL(request.url()).pathname.startsWith('/api/set_project_active')) statusRequests.push(request.postDataJSON());
  });
  const header = page.getByRole('region', { name: 'Project details', exact: true });
  const title = page.getByRole('heading', { level: 1, name: '[HDR-1] Header project', exact: true });
  const trigger = page.locator('#project-detail-actions-trigger');
  const dialog = page.getByRole('dialog');
  const open = async item => {
    await trigger.click();
    await page.locator('#project-detail-actions').getByRole('menuitem', { name: item, exact: true }).click();
    await expect(dialog).toBeVisible();
  };
  // Compare rectangles from one coordinate space, so CSS zoom cannot hide overflow.
  const overflowing = () => page.evaluate(() => {
    const root = document.documentElement, frame = root.getBoundingClientRect();
    const controls = [document.querySelector('h1'), ...['project-detail-back', 'project-detail-edit', 'project-detail-actions-trigger']
      .map(id => document.getElementById(id))].filter(Boolean);
    const outside = controls.filter(el => { const r = el.getBoundingClientRect(); return r.left < frame.left - 1 || r.right > frame.right + 1; })
      .map(el => el.id || el.tagName);
    if (root.scrollWidth > root.clientWidth + 1) outside.push('document');
    return outside;
  });
  // Each denied attempt is observed through its own response, not an earlier alert.
  const confirmDenied = async label => {
    const response = page.waitForResponse(r => new URL(r.url()).pathname.startsWith('/api/set_project_active'));
    await dialog.getByRole('button', { name: label, exact: true }).click();
    assert.equal((await response).status(), 403);
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('alert')).toContainText('Project details are unavailable');
    await expect(header).toHaveCount(0);
    await expect(trigger).toHaveCount(0);
    await expect(page.locator('.toast')).toHaveCount(0);
    await expect(page.locator('#project-detail-back')).toBeFocused();
  };
  const retry = async () => {
    await page.locator('#project-detail-retry').click();
    await expect(title).toBeVisible();
  };
  try {
    const login = await context.request.post(`${base}/auth/dev-login`, { maxRedirects: 0 });
    assert.equal(login.status(), 303);
    sql(`BEGIN;
      INSERT INTO clients (id,org_id,name,currency) VALUES ('${client}','${org}','Header client','EUR');
      INSERT INTO projects (id,org_id,client_id,code,name,project_type,currency,starts_on)
        VALUES ('${project}','${org}','${client}','HDR-1','Header project','fixed_fee','EUR','2026-09-01');
      INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ('${uncoded}','${org}','${client}','Uncoded header project','USD');
      INSERT INTO project_private_settings (id,org_id,project_id,admin_notes) VALUES ('${id(5)}','${org}','${project}','Header private note');
      INSERT INTO project_tags (id,org_id,name) VALUES ('${tag}','${org}','header-tag');
      INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ('${id(6)}','${org}','${project}','${tag}');
      COMMIT;`);

    await page.goto(`${base}/projects/${project}`);
    await expect(title).toBeVisible();
    await expect(header.getByRole('link', { name: 'Header client', exact: true })).toHaveAttribute('href', `/clients/${client}`);
    await expect(header).toContainText('Fixed Fee');
    await expect(header).toContainText('EUR');
    await expect(header).not.toContainText('Archived');
    await expect(page.getByRole('link', { name: 'Back to Projects', exact: true })).toHaveAttribute('href', '/projects');
    await expect(header).toContainText('Header private note');
    await expect(header).toContainText('header-tag');
    await expect(header).toContainText('01 Sep 2026');
    await expect(page.getByRole('region', { name: 'Project tasks', exact: true })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Project team', exact: true })).toBeVisible();
    console.log('PASS: header shows the real client link, code/name, type, currency and saved information');

    // Editing from the detail returns there; only this project's own detail path is honoured.
    const editLink = page.getByRole('link', { name: 'Edit project', exact: true });
    const editUrl = `${base}/projects/${project}/edit?return_to=/projects/${project}`;
    const editorClean = page.locator('[data-project-edit-state="clean"]');
    const cancelTo = async (url, destination) => {
      await page.goto(url);
      await expect(page.getByRole('heading', { level: 1, name: 'Edit project', exact: true })).toBeVisible();
      await expect(editorClean).toHaveCount(1);
      await page.locator('#np-cancel').click();
      await expect(page).toHaveURL(destination);
    };
    await cancelTo(editUrl, `${base}/projects/${project}`);
    await expect(title).toBeVisible();
    await expect(editLink).toHaveAttribute('href', `/projects/${project}/edit?return_to=/projects/${project}`);
    for (const exit of ['Cancel', 'Back']) {
      await editLink.click();
      await expect(page).toHaveURL(editUrl);
      if (exit === 'Cancel') await page.reload();
      await expect(editorClean).toHaveCount(1);
      if (exit === 'Cancel') {
        await page.locator('#np-cancel').click();
      } else {
        await expect(page.getByRole('button', { name: 'Back to Projects', exact: true })).toHaveCount(0);
        await page.getByRole('button', { name: 'Back to project', exact: true }).click();
      }
      await expect(page).toHaveURL(`${base}/projects/${project}`);
      await expect(title).toBeVisible();
    }

    // Dirty edits keep their guards: staying keeps the draft, browser Back asks first, Discard returns here.
    const editorName = page.locator('.np-page #np-name');
    const discardDialog = page.locator('#np-discard-dialog');
    const savedName = () => sql(`SELECT name FROM projects WHERE id='${project}'`);
    await editLink.click();
    await expect(editorName).toHaveValue('Header project');
    await editorName.fill('Unsaved header name');
    await page.locator('#np-cancel').click();
    await expect(discardDialog).toBeVisible();
    await discardDialog.getByRole('button', { name: 'Keep editing', exact: true }).click();
    await expect(discardDialog).toBeHidden();
    await expect(page).toHaveURL(editUrl);
    await expect(editorName).toHaveValue('Unsaved header name');
    const guard = page.waitForEvent('dialog', { timeout: 5000 });
    const historyBack = page.evaluate(() => history.back());
    await (await guard).dismiss();
    await historyBack;
    await expect(page).toHaveURL(editUrl);
    await expect(editorName).toHaveValue('Unsaved header name');
    await page.getByRole('button', { name: 'Back to project', exact: true }).click();
    await discardDialog.getByRole('button', { name: 'Discard changes', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${project}`);
    await expect(title).toBeVisible();
    assert.equal(savedName(), 'Header project', 'Discarding saves nothing');

    // Save stays explicit and lands on the saved project's detail, through a reload.
    for (const [from, name] of [['Header project', 'Saved header name'], ['Saved header name', 'Header project']]) {
      await editLink.click();
      await expect(editorName).toHaveValue(from);
      await editorName.fill(name);
      await page.locator('.np-page').getByRole('button', { name: 'Save changes', exact: true }).click();
      await expect(page).toHaveURL(`${base}/projects/${project}`);
      await expect(page.getByRole('heading', { level: 1, name: `[HDR-1] ${name}`, exact: true })).toBeVisible();
      assert.equal(savedName(), name);
      await page.reload();
      await expect(page.getByRole('heading', { level: 1, name: `[HDR-1] ${name}`, exact: true })).toBeVisible();
    }
    for (const forged of [`/projects/${uncoded}`, `//example.com/projects/${project}`, `https://example.com/projects/${project}`,
      `/projects/${project}/edit`, `/projects/${project}%3Fx=1`, 'javascript:alert(1)']) {
      await cancelTo(`${base}/projects/${project}/edit?return_to=${forged}`, `${base}/projects`);
    }
    await cancelTo(`${base}/projects/${project}/edit`, `${base}/projects`);
    assert.equal(savedName(), 'Header project', 'Leaving a clean editor saves nothing');
    await page.goto(`${base}/projects/${project}`);
    await expect(title).toBeVisible();
    console.log('PASS: detail-origin Cancel, Back and Discard return to this project; dirty guards and Save are unchanged; forged and plain edit URLs return to Projects');

    const titleColors = {};
    for (const theme of ['dark', 'light']) {
      // CSS zoom scales the whole page; a 200% root font size enlarges text and rem-based spacing, not px.
      for (const [width, height, zoom, fontSize] of [[320, 900, 1, ''], [390, 844, 1, ''], [768, 900, 1, ''], [1440, 900, 1, ''],
        [1440, 360, 1, ''], [768, 900, 2, ''], [320, 900, 1, '200%'], [390, 844, 1, '200%']]) {
        const label = `${theme}/${width}x${height}/zoom ${zoom}/text ${fontSize || '100%'}`;
        console.log(`CHECK: header and confirmation at ${label}`);
        await page.setViewportSize({ width, height });
        await page.evaluate(({ theme, zoom, fontSize }) => {
          document.documentElement.dataset.theme = theme;
          document.documentElement.style.zoom = String(zoom);
          document.documentElement.style.fontSize = fontSize;
        }, { theme, zoom, fontSize });
        for (const control of [title, page.getByRole('link', { name: 'Edit project', exact: true }), trigger]) {
          await control.scrollIntoViewIfNeeded();
          await expect(control).toBeVisible();
        }
        assert.deepEqual(await overflowing(), [], `${label}: overflow`);
        assert.equal(await header.evaluate(el => el.scrollWidth <= el.clientWidth + 1), true, `${label}: header overflow`);
        if (width === 1440 && height === 900) titleColors[theme] = await title.evaluate(el => getComputedStyle(el).color);
        if (zoom === 1 && !fontSize && height >= 844 && [390, 1440].includes(width)) {
          // The handoff's header controls are 44px tall.
          for (const control of [page.getByRole('link', { name: 'Edit project', exact: true }), trigger]) {
            const box = await control.boundingBox();
            assert.ok(Math.abs(box.height - 44) <= 1, `${label}: control height ${box.height}`);
          }
        }
        await open('Archive');
        assert.equal(await dialog.locator('.modal').evaluate(el => el.scrollWidth <= el.clientWidth + 1), true, `${label}: dialog overflow`);
        if (zoom === 1 && !fontSize && height === 900 && [320, 1440].includes(width)) {
          const padding = await dialog.locator('.modal-body').evaluate(el => { const s = getComputedStyle(el); return [s.paddingLeft, s.paddingRight]; });
          const expected = width === 320 ? '12px' : '24px';
          assert.deepEqual(padding, [expected, expected], `${label}: dialog body padding`);
        }
        for (const name of ['Archive project', 'Cancel']) {
          const action = dialog.getByRole('button', { name, exact: true });
          await action.scrollIntoViewIfNeeded();
          await expect(action).toBeInViewport({ ratio: 1 });
        }
        await page.keyboard.press('Escape');
        await expect(dialog).toBeHidden();
      }
    }
    assert.notEqual(titleColors.dark, titleColors.light, 'The title follows the theme tokens');
    await page.evaluate(() => {
      Object.assign(document.documentElement.style, { zoom: '', fontSize: '' });
      document.documentElement.dataset.theme = 'dark';
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    assert.equal(active(), 't');
    assert.equal(statusRequests.length, 0);
    console.log('PASS: header and confirmation fit at 320–1440px, a short viewport, 200% zoom and 200% text in both themes');

    await page.locator('#project-detail-back').focus();
    for (let step = 0; step < 8 && await page.evaluate(() => document.activeElement?.id) !== 'project-detail-actions-trigger'; step++) {
      await page.keyboard.press('Tab');
    }
    await expect(trigger).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(page.locator('#project-detail-actions').getByRole('menuitem', { name: 'Archive', exact: true })).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(dialog.getByRole('heading', { name: 'Archive this project?', exact: true })).toBeVisible();
    await page.keyboard.press('Tab');
    await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(dialog).toBeHidden();
    await expect(trigger).toBeFocused();
    await open('Archive');
    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await expect(trigger).toBeFocused();
    assert.equal(active(), 't');
    assert.equal(statusRequests.length, 0);
    const information = header.locator('details');
    await expect(header.getByText('header-tag', { exact: true })).toBeHidden();
    await page.keyboard.press('Tab');
    await expect(information.locator('summary')).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(information).toHaveJSProperty('open', true);
    for (const saved of ['01 Sep 2026', 'header-tag', 'Header private note']) {
      await expect(information.getByText(saved)).toBeVisible();
    }
    console.log('PASS: keyboard reaches Actions and Project information; cancel and Escape leave the project unchanged and return focus');

    await page.goto(`${base}/projects/${uncoded}`);
    await expect(page.getByRole('heading', { level: 1, name: 'Uncoded header project', exact: true })).toBeVisible();
    await expect(header).toContainText('Time & Materials');
    await expect(header).toContainText('USD');
    await page.getByRole('link', { name: 'Back to Projects', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.goto(`${base}/projects/${project}`);
    await page.getByRole('link', { name: 'Edit project', exact: true }).click();
    await expect(page).toHaveURL(editUrl);
    await page.goBack();
    await expect(title).toBeVisible();
    await header.getByRole('link', { name: 'Header client', exact: true }).click();
    await expect(page).toHaveURL(`${base}/clients/${client}`);
    await page.goBack();
    await expect(title).toBeVisible();
    console.log('PASS: back, edit and client navigation use the existing routes');

    await page.route('**/api/set_project_active*', route => route.abort(), { times: 1 });
    await open('Archive');
    const aborted = page.waitForEvent('requestfailed', r => new URL(r.url()).pathname.startsWith('/api/set_project_active'));
    await dialog.getByRole('button', { name: 'Archive project', exact: true }).click();
    await aborted;
    await expect(dialog.getByRole('alert')).toContainText('Could not update project status');
    await expect(dialog).toBeVisible();
    await expect(title).toBeVisible();
    assert.equal(active(), 't');

    let release;
    const held = new Promise(resolve => { release = resolve; });
    await page.route('**/api/set_project_active*', async route => { await held; await route.continue(); }, { times: 1 });
    const before = statusRequests.length;
    const archived = page.waitForResponse(r => new URL(r.url()).pathname.startsWith('/api/set_project_active'));
    await dialog.getByRole('button', { name: 'Archive project', exact: true }).click();
    const pending = dialog.getByRole('button', { name: 'Updating project…', exact: true });
    await expect(pending).toBeDisabled();
    await expect(dialog.getByRole('alert')).toHaveCount(0);
    await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    await pending.click({ force: true });
    await page.keyboard.press('Escape');
    await expect(dialog).toBeVisible();
    assert.equal(statusRequests.length, before + 1, 'A pending confirmation sends one request');
    release();
    assert.equal((await archived).status(), 200);
    await expect(dialog).toBeHidden();
    const archivedToast = page.getByRole('status').filter({ hasText: 'Project archived.' });
    await expect(archivedToast).toBeInViewport({ ratio: 1 });
    // The fixed container's toast keeps the token minimum (280px) at desktop width, in both themes.
    assert.equal(page.viewportSize().width, 1440);
    for (const theme of ['light', 'dark']) {
      await page.evaluate(theme => { document.documentElement.dataset.theme = theme; }, theme);
      const toastWidth = (await archivedToast.boundingBox()).width;
      assert.ok(toastWidth >= 280, `${theme}: toast width ${toastWidth}`);
    }
    await expect(header).toContainText('Archived');
    await expect(trigger).toBeFocused();
    assert.equal(active(), 'f');
    assert.deepEqual(statusRequests.at(-1).expected_requester, { org_id: org, user_id: actor.id });
    // A refresh that loses access must not leave the previous success beside the unavailable state.
    await page.route('**/api/get_project_detail_view*', route => route.fulfill({ status: 403, contentType: 'text/plain', body: 'Forbidden' }), { times: 1 });
    const deniedRefresh = page.waitForResponse(r => new URL(r.url()).pathname.startsWith('/api/get_project_detail_view'));
    await page.locator('#project-detail-refresh').click();
    assert.equal((await deniedRefresh).status(), 403);
    await expect(page.getByRole('alert')).toContainText('Project details are unavailable');
    await expect(page.locator('.toast')).toHaveCount(0);
    // The focused Refresh button unmounts; focus should land on a recovery control, not the page body.
    await expect.poll(() => page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName),
      { message: 'Focus after a denied refresh' }).toMatch(/^project-detail-(retry|back)$/);
    await retry();
    await expect(header).toContainText('Archived');
    await page.reload();
    await expect(title).toBeVisible();
    await expect(header).toContainText('Archived');
    await expect(header).toContainText('Header private note');
    await expect(header).toContainText('header-tag');
    console.log('PASS: a failed transport retries in place; pending and duplicate requests send once; archived state persists');

    await page.route('**/api/set_project_active*', async route => {
      const body = route.request().postDataJSON();
      body.expected_requester.user_id = id(99);
      await route.continue({ postData: JSON.stringify(body) });
    }, { times: 1 });
    await open('Reactivate');
    await confirmDenied('Reactivate project');
    assert.equal(active(), 'f', 'A request bound to another account changes nothing');
    await retry();

    sql(`UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'`);
    try {
      // The dialog was opened under policy zero; the write rechecks the policy.
      await open('Reactivate');
      await confirmDenied('Reactivate project');
      assert.equal(active(), 'f', 'A policy change while the dialog is open changes nothing');
    } finally {
      sql(`UPDATE organizations SET permission_policy_version=0 WHERE id='${org}'`);
    }
    await retry();

    await open('Reactivate');
    sql(`UPDATE users SET org_role='member' WHERE id='${actor.id}'`);
    try {
      await confirmDenied('Reactivate project');
      assert.equal(active(), 'f', 'Revoked status authority changes nothing');
      sql(`INSERT INTO assignments (id,project_id,user_id) VALUES ('${id(7)}','${project}','${actor.id}')`);
      await retry();
      await expect(trigger).toHaveCount(0);
      await expect(page.getByRole('link', { name: 'Edit project', exact: true })).toHaveCount(0);
      await expect(header.getByRole('link', { name: 'Header client', exact: true })).toBeVisible();
    } finally {
      sql(`UPDATE users SET org_role='admin' WHERE id='${actor.id}'`);
      sql(`DELETE FROM assignments WHERE id='${id(7)}'`);
    }
    await page.locator('#project-detail-refresh').click();
    await expect(trigger).toBeVisible();

    // A successful write whose reload is denied shows no success beside the unavailable state.
    await open('Reactivate');
    await page.route('**/api/get_project_detail_view*', route => route.fulfill({ status: 403, contentType: 'text/plain', body: 'Forbidden' }), { times: 1 });
    const written = page.waitForResponse(r => new URL(r.url()).pathname.startsWith('/api/set_project_active'));
    const deniedReload = page.waitForResponse(r => new URL(r.url()).pathname.startsWith('/api/get_project_detail_view'));
    await dialog.getByRole('button', { name: 'Reactivate project', exact: true }).click();
    assert.equal((await written).status(), 200);
    assert.equal((await deniedReload).status(), 403);
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('alert')).toContainText('Project details are unavailable');
    await expect(page.locator('.toast')).toHaveCount(0);
    await expect(page.locator('#project-detail-back')).toBeFocused();
    assert.equal(active(), 't');
    await retry();
    await expect(header).not.toContainText('Archived');
    await open('Archive');
    const rearchived = page.waitForResponse(r => new URL(r.url()).pathname.startsWith('/api/set_project_active'));
    await dialog.getByRole('button', { name: 'Archive project', exact: true }).click();
    assert.equal((await rearchived).status(), 200);
    await expect(header.getByText('Archived', { exact: true })).toBeVisible();
    assert.equal(active(), 'f');

    // 320px with a 200% root font size (text and rem spacing): archived, reactivated, then a new action.
    await page.setViewportSize({ width: 320, height: 700 });
    await page.evaluate(() => { document.documentElement.style.fontSize = '200%'; });
    await expect(header.getByText('Archived', { exact: true })).toBeVisible();
    assert.deepEqual(await overflowing(), [], '320px/200% archived overflow');
    await trigger.scrollIntoViewIfNeeded();
    await open('Reactivate');
    for (const name of ['Reactivate project', 'Cancel']) {
      const action = dialog.getByRole('button', { name, exact: true });
      await action.scrollIntoViewIfNeeded();
      await expect(action).toBeInViewport({ ratio: 1 });
    }
    const reactivated = page.waitForResponse(r => new URL(r.url()).pathname.startsWith('/api/set_project_active'));
    await dialog.getByRole('button', { name: 'Reactivate project', exact: true }).click();
    assert.equal((await reactivated).status(), 200);
    await expect(dialog).toBeHidden();
    const toast = page.getByRole('status').filter({ hasText: 'Project reactivated.' });
    await expect(toast).toBeInViewport({ ratio: 1 });
    await expect(header).not.toContainText('Archived');
    await expect(trigger).toBeFocused();
    assert.deepEqual(await overflowing(), [], '320px/200% reactivation overflow');
    assert.equal(active(), 't');
    // A new action replaces the previous result rather than showing beside it.
    await open('Archive');
    await expect(page.locator('.toast')).toHaveCount(0);
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    await page.setViewportSize({ width: 1440, height: 900 });
    assert.deepEqual(errors, []);
    console.log('PASS: changed requester, policy and role fail closed and hide the project; Retry recovers the bound view; reactivation persists');
  } catch (error) {
    console.error(error);
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot({ timeout: 3000 }).catch(failure => failure.message) });
    throw error;
  } finally {
    await browser.close();
    sql(`BEGIN;
      UPDATE organizations SET permission_policy_version=0 WHERE id='${org}';
      UPDATE users SET org_role='admin' WHERE id='${actor.id}';
      DELETE FROM assignments WHERE project_id IN ('${project}','${uncoded}');
      DELETE FROM project_tag_links WHERE project_id='${project}';
      DELETE FROM project_tags WHERE id='${tag}';
      DELETE FROM project_private_settings WHERE project_id='${project}';
      DELETE FROM projects WHERE id IN ('${project}','${uncoded}');
      DELETE FROM clients WHERE id='${client}'; COMMIT;`);
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
