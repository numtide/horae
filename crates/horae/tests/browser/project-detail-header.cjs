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
  const toastAction = name => page.locator('.toast').getByRole('button', { name, exact: true });
  const undo = toastAction('Undo');
  const isStatusWrite = r => new URL(r.url()).pathname.startsWith('/api/set_project_active');
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
    // Check the hydrated editor's Back label, optionally again after a reload of the
    // same address, then that Cancel really leaves for the expected destination.
    const cancelTo = async (url, destination, reload = false) => {
      const label = destination === `${base}/projects` ? 'Back to Projects' : 'Back to project';
      const ready = async () => {
        await expect(page.getByRole('heading', { level: 1, name: 'Edit project', exact: true })).toBeVisible();
        await expect(editorClean).toHaveCount(1);
        await expect(page.locator('.np-page').getByRole('button', { name: label, exact: true })).toBeVisible();
      };
      await page.goto(url, { waitUntil: 'networkidle' });
      await ready();
      if (reload) {
        await page.reload({ waitUntil: 'networkidle' });
        await ready();
      }
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
    // One standard URL decoding yields this project. A second encoding, an encoded
    // separator or any extra or repeated parameter fails closed. The browser keeps
    // the typed address, so a reload must reach the same label and exit again.
    await cancelTo(`${base}/projects/${project}/edit?return_to=%2Fprojects%2F${project}`, `${base}/projects/${project}`, true);
    const doubleEncoded = `return_to=%252Fprojects%252F${project}`;
    for (const query of [doubleEncoded, `return_to=%25252Fprojects%25252F${project}`,
      `return_to=/projects/${project}%26x=1`, `return_to=/projects/${project}&x=1`,
      `return_to=/projects/${project}&return_to=/projects/${project}`, `x=1&return_to=/projects/${project}`]) {
      await cancelTo(`${base}/projects/${project}/edit?${query}`, `${base}/projects`, query === doubleEncoded);
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
      // H102: the archive result offers a direct Undo beside Dismiss.
      for (const name of ['Undo', 'Dismiss']) {
        await expect(archivedToast.getByRole('button', { name, exact: true })).toBeInViewport({ ratio: 1 });
      }
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

    // An archived client refuses its project's reactivation; the dialog says why
    // and links to the client instead of offering a retry (client lifecycle US2).
    sql(`UPDATE clients SET active = false WHERE id = '${client}'`);
    try {
      await open('Reactivate');
      const refused = page.waitForResponse(isStatusWrite);
      await dialog.getByRole('button', { name: 'Reactivate project', exact: true }).click();
      assert.equal((await refused).status(), 409);
      await expect(dialog.getByRole('alert')).toContainText('Restore the client before restoring this project.');
      await expect(dialog.getByRole('alert')).not.toContainText('retry safely');
      assert.equal(await dialog.getByRole('link', { name: 'Open the client', exact: true }).getAttribute('href'), `/clients/${client}`);
      console.log(`OBSERVE: focus after a refused reactivation is ${await page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName)}`);
      const refusalText = page.getByText('Restore the client before restoring this project.');
      await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
      await expect(dialog).toBeHidden();
      // A dialog refusal ends with the dialog, by Cancel or by Escape.
      await expect(refusalText).toHaveCount(0);
      await open('Reactivate');
      await dialog.getByRole('button', { name: 'Reactivate project', exact: true }).click();
      await expect(dialog.getByRole('alert')).toContainText('Restore the client before restoring this project.');
      await page.keyboard.press('Escape');
      await expect(dialog).toBeHidden();
      await expect(refusalText).toHaveCount(0);
      assert.equal(active(), 'f');

      // Undo meets the same refusal: the Undo notice gives way to the reason.
      sql(`UPDATE clients SET active = true WHERE id = '${client}'`);
      sql(`UPDATE projects SET active = true WHERE id = '${project}'`);
      await page.reload();
      await expect(title).toBeVisible();
      await open('Archive');
      await dialog.getByRole('button', { name: 'Archive project', exact: true }).click();
      await expect(dialog).toBeHidden();
      await expect(undo).toBeVisible();
      sql(`UPDATE clients SET active = false WHERE id = '${client}'`);
      const undone = page.waitForResponse(isStatusWrite);
      await undo.click();
      assert.equal((await undone).status(), 409);
      await expect(undo).toHaveCount(0);
      const refusalAlert = page.getByRole('alert').filter({ hasText: 'Restore the client before restoring this project.' });
      await expect(refusalAlert).toBeVisible();
      await expect(refusalAlert).not.toContainText('Retry');
      assert.equal(await refusalAlert.getByRole('link', { name: 'Open the client', exact: true }).getAttribute('href'), `/clients/${client}`);
      // Focus and placement after the Undo refusal are for the native pass to judge.
      console.log(`OBSERVE: focus after a refused Undo is ${await page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName)}; alert top ${Math.round((await refusalAlert.boundingBox()).y)}px`);
      assert.equal(active(), 'f');
    } finally {
      sql(`UPDATE clients SET active = true WHERE id = '${client}'`);
    }
    await page.reload();
    await expect(title).toBeVisible();
    console.log('PASS: reactivating a project of an archived client is refused with the reason and a client link, from the dialog and from Undo');

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
    for (const name of ['Undo', 'Dismiss']) await expect(toastAction(name)).toBeInViewport({ ratio: 1 });
    // With an action beside it, the message must stay at least one word wide rather than
    // collapsing to a letter column; the handoff's action is semibold text.
    const [messageWidth, longestWord] = await page.locator('.toast .toast-msg').evaluate(el => {
      const style = getComputedStyle(el), ctx = document.createElement('canvas').getContext('2d');
      ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
      return [el.getBoundingClientRect().width, Math.max(...el.textContent.trim().split(/\s+/).map(word => ctx.measureText(word).width))];
    });
    assert.ok(messageWidth + 1 >= longestWord, `320px/200% toast message ${messageWidth}px is narrower than a word (${longestWord}px)`);
    assert.equal(await undo.evaluate(el => getComputedStyle(el).fontWeight), '600', 'Undo uses the handoff weight');
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
    await expect(undo).toHaveCount(0);
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
    console.log('PASS: changed requester, policy and role fail closed and hide the project; Retry recovers the bound view; reactivation persists');

    // Undo reactivates directly under current authority and changes nothing but the status.
    const persisted = () => sql(`SELECT row_to_json(p) FROM (SELECT client_id,code,name,project_type,currency,rate_cents,
        starts_on,ends_on,budget_kind,budget_amount_cents,budget_minutes,created_at,
        (SELECT admin_notes FROM project_private_settings s WHERE s.project_id=projects.id) notes,
        (SELECT array_agg(tag_id) FROM project_tag_links l WHERE l.project_id=projects.id) tags
      FROM projects WHERE id='${project}') p`);
    // xmin changes only when the row is rewritten, so it proves an idempotent retry wrote nothing.
    const rowVersion = () => sql(`SELECT xmin FROM projects WHERE id='${project}'`);
    const content = persisted();
    const failure = page.getByText('Could not confirm restore. Retry Undo.', { exact: true });
    const settle = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const archive = async () => {
      await open('Archive');
      const done = page.waitForResponse(isStatusWrite);
      await dialog.getByRole('button', { name: 'Archive project', exact: true }).click();
      assert.equal((await done).status(), 200);
      await expect(dialog).toBeHidden();
      await expect(header.getByText('Archived', { exact: true })).toBeVisible();
      await expect(undo).toBeEnabled();
      assert.equal(active(), 'f');
    };
    const reactivate = async () => {
      await open('Reactivate');
      const done = page.waitForResponse(isStatusWrite);
      await dialog.getByRole('button', { name: 'Reactivate project', exact: true }).click();
      assert.equal((await done).status(), 200);
      await expect(page.getByRole('status').filter({ hasText: 'Project reactivated.' })).toBeVisible();
      await expect(undo).toHaveCount(0);
      assert.equal(active(), 't');
    };

    await archive();
    let releaseUndo;
    const undoHeld = new Promise(resolve => { releaseUndo = resolve; });
    await page.route('**/api/set_project_active*', async route => { await undoHeld; await route.continue(); }, { times: 1 });
    let sent = statusRequests.length;
    const undone = page.waitForResponse(isStatusWrite);
    await undo.focus();
    await page.keyboard.press('Enter');
    const pendingUndo = toastAction('Updating project…');
    await expect(pendingUndo).toBeDisabled();
    await expect(trigger).toBeDisabled();
    await expect(dialog).toBeHidden();
    await page.keyboard.press('Enter');
    await expect(dialog).toBeHidden();
    await settle();
    assert.equal(statusRequests.length, sent + 1, 'A pending Undo sends one request');
    assert.equal(statusRequests.at(-1).active, true);
    assert.deepEqual(statusRequests.at(-1).expected_requester, { org_id: org, user_id: actor.id });
    releaseUndo();
    assert.equal((await undone).status(), 200);
    await expect(header).not.toContainText('Archived');
    await expect(undo).toHaveCount(0);
    await expect(dialog).toBeHidden();
    await expect(trigger).toBeFocused();
    assert.equal(active(), 't');
    assert.equal(persisted(), content, 'Undo changes only the status');
    await page.reload();
    await expect(title).toBeVisible();
    await expect(header).not.toContainText('Archived');
    await expect(header).toContainText('Header private note');
    await expect(page.locator('.toast')).toHaveCount(0);
    console.log('PASS: keyboard Undo reactivates once without a second confirmation; pending duplicates are suppressed; the result persists');

    // Disabled buttons take no pointer events, so forced clicks fall through to the page;
    // they must still send nothing, and a reload must not steal focus the user moved.
    await archive();
    let releasePointer;
    const pointerHeld = new Promise(resolve => { releasePointer = resolve; });
    await page.route('**/api/set_project_active*', async route => { await pointerHeld; await route.continue(); }, { times: 1 });
    const sidebarProjects = page.locator('#app-navigation').getByRole('link', { name: 'Projects', exact: true });
    try {
      sent = statusRequests.length;
      const pointerUndo = page.waitForResponse(isStatusWrite);
      await undo.click();
      await expect(pendingUndo).toBeDisabled();
      await pendingUndo.click({ force: true });
      await trigger.click({ force: true });
      await expect(dialog).toBeHidden();
      await settle();
      assert.equal(statusRequests.length, sent + 1, 'Forced clicks on a pending Undo send nothing more');
      await sidebarProjects.focus();
      releasePointer();
      assert.equal((await pointerUndo).status(), 200);
    } finally {
      releasePointer();
    }
    await expect(header).not.toContainText('Archived');
    await expect(undo).toHaveCount(0);
    await settle();
    await expect(sidebarProjects).toBeFocused();
    assert.equal(active(), 't');
    assert.equal(persisted(), content);
    console.log('PASS: forced pointer duplicates send one request; the reload keeps deliberately moved focus');

    await archive();
    sent = statusRequests.length;
    await toastAction('Dismiss').focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('.toast')).toHaveCount(0);
    await settle();
    assert.equal(statusRequests.length, sent, 'Dismissal writes nothing');
    assert.equal(active(), 'f');
    await reactivate();
    assert.equal(persisted(), content);
    console.log('PASS: Dismiss removes Undo without writing; a later Reactivate works and offers no inverse Undo');

    // Dismissing a pending Undo must not hide its later failure, or the archive has no visible retry.
    await archive();
    // The whole toast and each action's content stay on screen at a short 320px with 200% text.
    const toastFits = async state => {
      await expect(page.locator('.toast')).toBeInViewport({ ratio: 1 });
      const spilled = await page.locator('.toast').evaluate(toast => {
        const style = getComputedStyle(toast), box = toast.getBoundingClientRect();
        const left = box.left + parseFloat(style.borderLeftWidth) + parseFloat(style.paddingLeft);
        const right = box.right - parseFloat(style.borderRightWidth) - parseFloat(style.paddingRight);
        return [...toast.querySelectorAll('button')].filter(button => {
          const r = button.getBoundingClientRect();
          return r.left < left - 1 || r.right > right + 1 || button.scrollWidth > button.clientWidth + 1;
        }).map(button => button.textContent.trim() || button.getAttribute('aria-label'));
      });
      assert.deepEqual(spilled, [], `320x568/200% ${state}: toast actions spill`);
    };
    await page.setViewportSize({ width: 320, height: 568 });
    await page.evaluate(() => { document.documentElement.style.fontSize = '200%'; });
    let failHeld;
    const dismissHeld = new Promise(resolve => { failHeld = resolve; });
    await page.route('**/api/set_project_active*', async route => { await dismissHeld; await route.abort(); }, { times: 1 });
    try {
      const abandoned = page.waitForEvent('requestfailed', isStatusWrite);
      await undo.focus();
      await page.keyboard.press('Enter');
      await expect(toastAction('Updating project…')).toBeDisabled();
      await toastFits('pending');
      await toastAction('Dismiss').focus();
      await page.keyboard.press('Enter');
      failHeld();
      await abandoned;
    } finally {
      failHeld();
    }
    await expect(dialog).toBeHidden();
    await expect(failure).toBeVisible();
    await expect(undo).toBeEnabled();
    await toastFits('failure');
    assert.equal(active(), 'f');
    const recovered = page.waitForResponse(isStatusWrite);
    await undo.focus();
    await page.keyboard.press('Enter');
    assert.equal((await recovered).status(), 200);
    await expect(header).not.toContainText('Archived');
    await expect(undo).toHaveCount(0);
    assert.equal(active(), 't');
    assert.equal(persisted(), content);
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    await page.setViewportSize({ width: 1440, height: 900 });
    console.log('PASS: a pending Undo dismissed before it fails still shows a retryable failure');

    await archive();
    await page.route('**/api/set_project_active*', route => route.abort(), { times: 1 });
    const failedUndo = page.waitForEvent('requestfailed', isStatusWrite);
    await undo.click();
    await failedUndo;
    await expect(dialog).toBeHidden();
    await expect(failure).toBeVisible();
    await expect(undo).toBeEnabled();
    assert.equal(active(), 'f');
    // The write commits but its response is lost; the retry converges without rewriting the row.
    await page.route('**/api/set_project_active*', async route => { await route.fetch(); await route.abort(); }, { times: 1 });
    const lostUndo = page.waitForEvent('requestfailed', isStatusWrite);
    await undo.focus();
    await page.keyboard.press('Enter');
    await lostUndo;
    await expect(failure).toBeVisible();
    await expect(undo).toBeEnabled();
    assert.equal(active(), 't', 'The unacknowledged write committed');
    const committed = rowVersion();
    const retriedUndo = page.waitForResponse(isStatusWrite);
    await undo.focus();
    await page.keyboard.press('Enter');
    assert.equal((await retriedUndo).status(), 200);
    await expect(header).not.toContainText('Archived');
    await expect(undo).toHaveCount(0);
    await expect(failure).toBeHidden();
    assert.equal(active(), 't');
    assert.equal(rowVersion(), committed, 'Retrying an already-active project rewrites nothing');
    assert.equal(persisted(), content);
    console.log('PASS: a failed or unacknowledged Undo stays visible and retryable outside the dialog and converges to active');

    const rebind = route => {
      const body = route.request().postDataJSON();
      body.expected_requester.user_id = id(99);
      return route.continue({ postData: JSON.stringify(body) });
    };
    for (const [change, revoke, restore] of [
      ['account', () => page.route('**/api/set_project_active*', rebind, { times: 1 }), () => {}],
      ['policy', () => sql(`UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'`),
        () => sql(`UPDATE organizations SET permission_policy_version=0 WHERE id='${org}'`)],
      ['role', () => sql(`UPDATE users SET org_role='member' WHERE id='${actor.id}'`),
        () => sql(`UPDATE users SET org_role='admin' WHERE id='${actor.id}'`)],
    ]) {
      await archive();
      await revoke();
      try {
        const denied = page.waitForResponse(isStatusWrite);
        await undo.click();
        assert.equal((await denied).status(), 403, `${change}: Undo is denied`);
        await expect(page.getByRole('alert')).toContainText('Project details are unavailable');
        await expect(header).toHaveCount(0);
        await expect(page.locator('.toast')).toHaveCount(0);
        await expect(dialog).toBeHidden();
        await expect.poll(() => page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName),
          { message: `${change}: focus after a denied Undo` }).toMatch(/^project-detail-(retry|back)$/);
        assert.equal(active(), 'f', `${change}: a denied Undo changes nothing`);
      } finally {
        await restore();
      }
      await retry();
      await expect(header.getByText('Archived', { exact: true })).toBeVisible();
      await expect(undo).toHaveCount(0);
      await reactivate();
    }
    assert.equal(persisted(), content);
    console.log('PASS: a changed account, policy or role denies Undo, writes nothing and removes the stale notice');

    // Leaving while Undo is pending carries neither its action nor its feedback to another project.
    await archive();
    let releaseAway;
    const awayHeld = new Promise(resolve => { releaseAway = resolve; });
    await page.route('**/api/set_project_active*', async route => { await awayHeld; await route.continue().catch(() => {}); }, { times: 1 });
    const awaySettled = Promise.race([page.waitForResponse(isStatusWrite), page.waitForEvent('requestfailed', isStatusWrite)]);
    await undo.click();
    await expect(toastAction('Updating project…')).toBeDisabled();
    await page.getByRole('link', { name: 'Back to Projects', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.getByRole('link', { name: 'Uncoded header project', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${uncoded}`);
    await expect(page.getByRole('heading', { level: 1, name: 'Uncoded header project', exact: true })).toBeVisible();
    releaseAway();
    await awaySettled;
    await settle();
    await expect(page.locator('.toast')).toHaveCount(0);
    await expect(failure).toBeHidden();
    await expect(header).not.toContainText('Archived');
    assert.equal(sql(`SELECT active FROM projects WHERE id='${uncoded}'`), 't');
    await page.goBack();
    await page.goBack();
    await expect(title).toBeVisible();
    await expect(page.locator('.toast')).toHaveCount(0);
    // Whether the abandoned request completed or not, the page shows the persisted status.
    if (active() === 'f') await expect(header.getByText('Archived', { exact: true })).toBeVisible();
    else await expect(header).not.toContainText('Archived');
    assert.equal(persisted(), content);
    console.log('PASS: navigating away during Undo isolates its action and feedback; the detail shows the persisted status');
    assert.deepEqual(errors, []);
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
