// Real canonical commands and browser storage, only in the runner's disposable DB.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const { mkdirSync } = require('node:fs');
const { join } = require('node:path');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id, org_id, name FROM users WHERE email='admin@example.com' AND active AND org_role='admin') u"));
assert.match(actor.id, /^[0-9a-f-]{36}$/);
assert.match(actor.org_id, /^[0-9a-f-]{36}$/);
assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${actor.org_id}'`), '0');
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${actor.org_id}'`), '0');
assert.equal(sql(`SELECT count(*) FROM permission_change_receipts WHERE org_id='${actor.org_id}'`), '0');
const originalRevision = sql(`SELECT access_revision FROM organizations WHERE id='${actor.org_id}'`);
assert.match(originalRevision, /^\d+$/);
const person = '01960000-0000-7000-8000-000000000701';
const actorState = '01960000-0000-7000-8000-000000000702';
const personState = '01960000-0000-7000-8000-000000000703';
const otherActor = '01960000-0000-7000-8000-000000000704';
const otherState = '01960000-0000-7000-8000-000000000705';
const personName = 'Permission browser fixture';
const templateName = 'Browser recovery profile';
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const administrator = [...floor,
  'time_read_managed', 'time_write_managed', 'time_approve_managed', 'time_read_all', 'time_write_all', 'time_approve_all',
  'expense_read_managed', 'expense_write_managed', 'expense_read_all', 'expense_write_all',
  'project_read_managed', 'project_write_managed', 'project_create_all', 'project_read_all', 'project_write_all',
  'client_read_all', 'client_write_all', 'task_read_all', 'task_write_all',
  'people_read_managed', 'people_write_managed', 'people_read_all', 'people_write_all',
  'billable_rate_read_managed', 'billable_rate_write_managed', 'billable_rate_read_all', 'billable_rate_write_all',
  'cost_rate_read_all', 'cost_rate_write_all', 'invoice_read_managed', 'invoice_draft_write_managed', 'invoice_write_managed',
  'invoice_read_all', 'invoice_write_all', 'estimate_read_all', 'estimate_write_all',
  'report_profitability_read', 'report_contractor_read', 'report_invoicing_read',
  'saved_report_read_inactive', 'saved_report_write_inactive', 'approval_withdraw_managed',
  'company_read', 'company_write', 'billing_read', 'billing_write'];
const key = `horae-permission-request:v1:${actor.org_id}:${actor.id}`;
const array = grants => `ARRAY[${grants.map(grant => `'${grant}'`).join(',')}]`;

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  console.log(`Permission recovery browser: Chromium ${browser.version()}`);
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const errors = [], saves = [], pending = new Set();
  let loseResponse = false, committedLostResponse = false, discard = false;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('dialog', dialog => dialog.type() === 'beforeunload' || discard ? dialog.accept() : dialog.dismiss());
  page.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) pending.add(request); });
  page.on('requestfinished', request => pending.delete(request));
  page.on('requestfailed', request => pending.delete(request));
  const readsFinished = () => expect.poll(() => pending.size).toBe(0);
  const record = () => page.evaluate(key => sessionStorage.getItem(key), key);
  const receipts = () => Number(sql(`SELECT count(*) FROM permission_change_receipts WHERE org_id='${actor.org_id}' AND actor_user_id='${actor.id}'`));
  const editor = page.getByRole('dialog', { name: 'Edit permissions', exact: true });
  const recovery = page.getByRole('dialog', { name: 'Recover permission request', exact: true });
  const openEditor = async (name = personName) => {
    await page.getByRole('button', { name: `Edit permissions for ${name}`, exact: true }).click();
    await expect(editor.getByRole('button', { name: 'Review changes', exact: true })).toBeEnabled();
    await readsFinished();
  };
  const review = async () => {
    await editor.getByRole('button', { name: 'Review changes', exact: true }).click();
    await expect(editor.getByRole('button', { name: 'Confirm permissions', exact: true })).toBeEnabled();
  };
  const capture = async label => {
    for (const [width, height, theme] of [[1440, 900, 'dark'], [390, 844, 'light']]) {
      await page.setViewportSize({ width, height });
      await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
      const dialog = page.locator('dialog[open]');
      await expect(dialog).toBeVisible();
      assert.equal(await dialog.evaluate(el => el.scrollWidth > el.clientWidth + 1), false, 'dialog must not overflow horizontally');
      const panel = await dialog.locator('.modal').boundingBox();
      assert.ok(panel && panel.x >= 0 && panel.x + panel.width <= width + 1);
      if (process.env.HORAE_BROWSER_ARTIFACTS) {
        mkdirSync(process.env.HORAE_BROWSER_ARTIFACTS, { recursive: true });
        await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, `${label}-${width}-${theme}.png`), animations: 'disabled' });
      }
    }
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.evaluate(() => document.documentElement.dataset.theme = 'dark');
  };
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname;
    if (/^\/api\/save_(person_permissions|permission_template)/.test(path)) {
      const body = route.request().postDataJSON();
      const stored = JSON.parse(await record());
      assert.deepEqual(body.expected_requester, { org_id: actor.org_id, user_id: actor.id });
      assert.deepEqual(stored.requester, body.expected_requester);
      assert.deepEqual(stored.command.value, body.command, 'storage acknowledgement must precede the real mutation');
      saves.push(body);
      if (loseResponse) {
        loseResponse = false;
        const response = await route.fetch();
        assert.equal(response.status(), 200, await response.text());
        committedLostResponse = true;
        return route.abort('failed');
      }
    }
    return route.continue();
  });
  try {
    sql(`BEGIN;
      INSERT INTO users (id, org_id, email, name) VALUES
        ('${person}', '${actor.org_id}', 'permission-browser@example.test', '${personName}'),
        ('${otherActor}', '${actor.org_id}', 'permission-other-browser@example.test', 'Other permission administrator');
      INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source, built_in_profile) VALUES
        ('${actorState}', '${actor.org_id}', '${actor.id}', 1, ${array(administrator)}, true, 'built_in', 'administrator'),
        ('${personState}', '${actor.org_id}', '${person}', 1, ${array(floor)}, false, 'built_in', 'member'),
        ('${otherState}', '${actor.org_id}', '${otherActor}', 1, ${array(administrator)}, true, 'built_in', 'administrator');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${actor.org_id}'; COMMIT;`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await readsFinished();
    await page.goto(`${base}/admin/users`);
    await openEditor();
    await capture('permission-editor');
    await editor.locator('#person-permissions-profile').focus();
    await page.keyboard.press('Tab');
    await expect(editor.locator('#permission-TimeReadManaged')).toBeFocused();
    await page.getByRole('button', { name: `Edit permissions for ${personName}`, exact: true, includeHidden: true }).evaluate(el => el.focus());
    await expect(editor.locator('#permission-TimeReadManaged')).toBeFocused();

    // A dirty keyboard dismissal must preserve the draft unless confirmed.
    await editor.locator('#permission-ClientReadAll').click();
    await page.keyboard.press('Escape');
    await expect(editor).toBeVisible();
    await expect(editor.locator('#permission-ClientReadAll')).toBeChecked();
    discard = true;
    await page.keyboard.press('Escape');
    await expect(editor).not.toBeVisible();
    discard = false;
    await expect(page.getByRole('button', { name: `Edit permissions for ${personName}`, exact: true })).toBeFocused();
    await openEditor();
    await expect(editor.locator('#permission-ClientReadAll')).not.toBeChecked();
    await editor.locator('#permission-ClientReadAll').click();
    await review();
    const count = receipts();
    loseResponse = true;
    await editor.getByRole('button', { name: 'Confirm permissions', exact: true }).click();
    await expect(editor.getByRole('button', { name: 'Retry same save', exact: true })).toBeEnabled();
    assert.equal(committedLostResponse, true);
    assert.equal(receipts(), count + 1);
    const firstRecord = await record();
    const original = structuredClone(saves.at(-1));
    await page.reload();
    await expect(recovery.getByRole('button', { name: 'Retry original request', exact: true })).toBeEnabled();
    await readsFinished();
    assert.equal(await record(), firstRecord);
    assert.equal(saves.length, 1, 'reload must not automatically replay a command');
    await capture('permission-recovery');
    await recovery.getByRole('button', { name: 'Retry original request', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(recovery).not.toBeVisible();
    assert.deepEqual(saves.at(-1), original);
    assert.equal(receipts(), count + 1);
    assert.equal(await record(), null);
    assert.equal(sql(`SELECT 'client_read_all'=ANY(grants) FROM person_permission_states WHERE user_id='${person}'`), 't');
    console.log('PASS: real person save survives lost response/reload and replays exactly one receipt');

    // A later loss of authority cannot erase a successfully committed request.
    await openEditor();
    await editor.locator('summary').filter({ hasText: 'Reusable custom profiles' }).click();
    await editor.locator('#permission-template-create').click();
    await editor.getByLabel('Profile name', { exact: true }).fill(templateName);
    loseResponse = true;
    await editor.locator('#permission-template-save').click();
    await expect(editor.getByRole('button', { name: 'Retry same save', exact: true })).toBeEnabled();
    const templateRecord = await record();
    const templateCommand = structuredClone(saves.at(-1));
    const templateCount = receipts();
    const callsBeforeSwitch = saves.length;
    await readsFinished();
    // Change only disposable login candidates; never offer another user's retained request.
    sql(`UPDATE users SET org_role='manager' WHERE id='${actor.id}'; UPDATE users SET org_role='admin' WHERE id='${otherActor}'`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await readsFinished();
    await page.goto(`${base}/admin/users`);
    await expect(page.getByRole('button', { name: `Edit permissions for ${personName}`, exact: true })).toBeVisible();
    await readsFinished();
    await expect(recovery).not.toBeVisible();
    assert.equal(await record(), templateRecord);
    assert.equal(saves.length, callsBeforeSwitch);
    sql(`UPDATE users SET org_role='admin' WHERE id='${actor.id}'; UPDATE users SET org_role='member' WHERE id='${otherActor}'`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await readsFinished();
    await page.goto(`${base}/admin/users`);
    await expect(recovery.getByRole('button', { name: 'Retry original request', exact: true })).toBeEnabled();
    assert.equal(await record(), templateRecord);
    sql(`UPDATE person_permission_states SET is_administrator=false WHERE id='${actorState}'`);
    await page.reload();
    await expect(recovery).toBeVisible();
    await recovery.locator('#permission-recovery-retry').click();
    await expect(recovery.getByText(/An earlier attempt may still have saved/)).toBeVisible();
    assert.equal(await record(), templateRecord);
    assert.equal(receipts(), templateCount);
    await expect(recovery.locator('#permission-recovery-discard')).toBeDisabled();
    sql(`UPDATE person_permission_states SET is_administrator=true WHERE id='${actorState}'`);
    await recovery.locator('#permission-recovery-retry').click();
    await expect(recovery.getByText(/Custom profile request completed/)).toBeVisible();
    assert.deepEqual(saves.at(-1), templateCommand);
    assert.equal(receipts(), templateCount);
    assert.equal(await record(), null);
    await recovery.getByRole('button', { name: 'Done', exact: true }).click();
    await expect(recovery).not.toBeVisible();
    await readsFinished();
    // The own-access resource was loaded while revoked; a fresh visit restores its entry action.
    await page.goto(`${base}/admin/users`);
    console.log('PASS: account changes isolate tab recovery; same-user reauthentication and restored authority recover the template');

    // The deleted template must not be needed to recover its historical receipt.
    const templateId = sql(`SELECT id FROM permission_templates WHERE org_id='${actor.org_id}' AND name='${templateName}'`);
    assert.match(templateId, /^[0-9a-f-]{36}$/);
    await openEditor();
    await editor.locator('summary').filter({ hasText: 'Reusable custom profiles' }).click();
    await editor.locator(`#permission-template-delete-${templateId}`).click();
    await expect(editor.getByText('No people currently use this profile.', { exact: true })).toBeVisible();
    await editor.locator('#permission-template-confirm').click();
    loseResponse = true;
    await editor.locator('#permission-template-save').click();
    await expect(editor.getByRole('button', { name: 'Retry same save', exact: true })).toBeEnabled();
    const deletion = structuredClone(saves.at(-1));
    const deletionCount = receipts();
    assert.equal(sql(`SELECT count(*) FROM permission_templates WHERE id='${templateId}'`), '0');
    await page.reload();
    await recovery.getByRole('button', { name: 'Retry original request', exact: true }).click();
    await expect(recovery.getByText(/Custom profile request completed/)).toBeVisible();
    assert.deepEqual(saves.at(-1), deletion);
    assert.equal(receipts(), deletionCount);
    assert.equal(sql(`SELECT count(*) FROM permission_templates WHERE org_id='${actor.org_id}' AND name='${templateName}'`), '0');
    assert.equal(await record(), null);
    await recovery.getByRole('button', { name: 'Done', exact: true }).click();
    await expect(recovery).not.toBeVisible();
    console.log('PASS: a deleted template is recovered from its exact receipt without recreating it');

    // Self-demotion makes any second server send forbidden; cleanup must stay local.
    await openEditor(actor.name);
    await editor.locator('#person-permissions-profile').selectOption('Member');
    await review();
    await page.evaluate(() => {
      const original = Storage.prototype.setItem;
      Storage.prototype.setItem = function(key, value) { if (key.startsWith('horae-permission-request:')) throw new DOMException('Test quota', 'QuotaExceededError'); return original.call(this, key, value); };
      window.restorePermissionStorage = () => { Storage.prototype.setItem = original; };
    });
    const callsBeforeFailure = saves.length;
    await editor.locator('#permission-save').click();
    await expect(editor.getByRole('alert')).toContainText('Browser session storage is unavailable');
    assert.equal(saves.length, callsBeforeFailure);
    await page.evaluate(() => {
      window.restorePermissionStorage();
      const original = Storage.prototype.removeItem;
      Storage.prototype.removeItem = function(key) { if (key.startsWith('horae-permission-request:')) throw new DOMException('Test storage failure'); return original.call(this, key); };
      window.restorePermissionStorage = () => { Storage.prototype.removeItem = original; };
    });
    await editor.locator('#permission-save').click();
    await expect(editor.getByRole('button', { name: 'Finish recovery cleanup', exact: true })).toBeEnabled();
    assert.equal(saves.length, callsBeforeFailure + 1);
    assert.equal(sql(`SELECT is_administrator FROM person_permission_states WHERE id='${actorState}'`), 'f');
    assert.notEqual(await record(), null);
    await page.evaluate(() => window.restorePermissionStorage());
    await editor.locator('#permission-save').click();
    await expect(editor).not.toBeVisible();
    assert.equal(saves.length, callsBeforeFailure + 1);
    assert.equal(await record(), null);
    await readsFinished();
    assert.deepEqual(errors, []);
    console.log('PASS: storage refusal prevents sends; acknowledged self-demotion cleanup issues no second mutation');
  } catch (error) {
    console.error('Browser failure state:', await page.evaluate(() => ({
      url: location.href,
      dialogs: [...document.querySelectorAll('dialog')].map(el => ({ id: el.id, open: el.open, busy: el.getAttribute('aria-busy'), text: el.innerText.slice(0, 1000) })),
      recovery: Object.keys(sessionStorage).filter(key => key.startsWith('horae-permission-request:')),
      focus: document.activeElement?.outerHTML.slice(0, 300),
    })));
    console.error('Browser errors:', errors);
    if (process.env.HORAE_BROWSER_ARTIFACTS) {
      await page.screenshot({ path: join(process.env.HORAE_BROWSER_ARTIFACTS, 'permission-failure.png') });
    }
    throw error;
  } finally {
    await context.close();
    await browser.close();
    sql(`BEGIN;
      SELECT id FROM organizations WHERE id='${actor.org_id}' FOR UPDATE;
      DELETE FROM permission_change_receipts WHERE org_id='${actor.org_id}' AND actor_user_id='${actor.id}';
      DELETE FROM person_permission_states WHERE id IN ('${actorState}', '${personState}', '${otherState}');
      DELETE FROM permission_templates WHERE org_id='${actor.org_id}' AND name='${templateName}';
      DELETE FROM users WHERE id='${person}' AND email='permission-browser@example.test';
      DELETE FROM users WHERE id='${otherActor}' AND email='permission-other-browser@example.test';
      UPDATE users SET org_role='admin' WHERE id='${actor.id}';
      UPDATE organizations SET permission_policy_version=0, access_revision=${originalRevision} WHERE id='${actor.org_id}'; COMMIT;`);
    assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE org_id='${actor.org_id}'`), '0');
    assert.equal(sql(`SELECT count(*) FROM permission_change_receipts WHERE org_id='${actor.org_id}'`), '0');
    assert.equal(sql(`SELECT permission_policy_version FROM organizations WHERE id='${actor.org_id}'`), '0');
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
