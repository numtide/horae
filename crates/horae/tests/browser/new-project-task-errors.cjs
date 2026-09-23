// Incomplete draft fixtures use the real save endpoint in the disposable runner.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const rowId = '01960000-0000-7000-8000-000000000201';
const missingPerson = '01960000-0000-7000-8000-000000000202';

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const pending = new Set(), errors = [];
  let saveRequest;
  page.on('pageerror', error => errors.push(error.stack || error.message));
  page.on('request', request => {
    if (!new URL(request.url()).pathname.startsWith('/api/')) return;
    pending.add(request);
    if (request.url().includes('/api/save_project_draft')) saveRequest = request;
  });
  page.on('requestfinished', request => pending.delete(request));
  page.on('requestfailed', request => pending.delete(request));
  const readsFinished = () => expect.poll(() => pending.size).toBe(0);
  const screen = page.locator('.np-page');
  const saved = () => expect(screen.locator('header').getByRole('status')).toContainText('Draft saved at');
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await readsFinished();
    await page.goto(`${base}/projects/new`);
    await screen.getByRole('button', { name: 'Client', exact: true }).click();
    await page.getByRole('dialog', { name: 'Choose Client', exact: true }).getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await screen.getByLabel('Project name', { exact: true }).fill('Task error recovery project');
    await saved();
    await readsFinished();
    const draftId = saveRequest.postDataJSON().draft_id;
    assert.match(draftId, /^[0-9a-f-]{36}$/);
    const draft = () => JSON.parse(sql(`SELECT row_to_json(d) FROM (SELECT revision, payload FROM project_drafts WHERE id = '${draftId}') d`));

    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      for (const kind of ['name', 'access']) {
        await readsFinished();
        await page.goto(`${base}/projects`);
        await readsFinished();
        const before = draft();
        const form = structuredClone(before.payload);
        form.tasks = [{ id: rowId, source: { kind: 'new', name: kind === 'name' ? 'é'.repeat(201) : 'Recoverable task' },
          billable: true, rate: '', budget: '', access: kind === 'access' ? { kind: 'restricted', user_ids: [missingPerson] } : { kind: 'everyone' } }];
        const response = await context.request.post(saveRequest.url(), {
          headers: { 'content-type': saveRequest.headers()['content-type'] },
          data: JSON.stringify({ draft_id: draftId, expected_revision: before.revision, form }),
        });
        assert.equal(response.status(), 200, await response.text());
        await page.goto(`${base}/projects/new`);
        await saved();
        await screen.getByRole('button', { name: 'Save project', exact: true }).focus();
        await page.keyboard.press('Enter');
        await expect(screen.locator('header').getByRole('status')).toHaveText('Changes need attention');
        const field = screen.locator(`#np-task-${kind === 'name' ? 'remove' : 'access'}-${rowId}`);
        const message = screen.locator(`#np-task-error-${rowId}`);
        await expect(field).toHaveAttribute('aria-invalid', 'true');
        await expect(field).toHaveAttribute('aria-describedby', `np-task-error-${rowId}`);
        await expect(field).toBeFocused();
        await expect(message).toContainText(kind === 'name' ? 'Task name' : 'Task access');
        await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Task error recovery project');
        assert.deepEqual(draft().payload, form, 'A rejected finalization must not rewrite the saved input');
        const footer = await screen.locator('.np-footer').boundingBox();
        for (const item of [field, message]) {
          const box = await item.boundingBox();
          assert.ok(box.y >= 0 && box.y + box.height <= footer.y, `${kind} recovery stays above actions at ${width}`);
        }
        await page.keyboard.press('Enter');
        if (kind === 'access') {
          const dialog = page.getByRole('dialog', { name: 'Who can track to this task?', exact: true });
          await dialog.getByRole('radio', { name: 'Everyone on the project', exact: true }).check();
          await dialog.getByRole('button', { name: 'Apply access', exact: true }).click();
          await expect(field).toBeFocused();
        } else {
          await expect(screen.locator('#np-task-search')).toBeFocused();
        }
        await screen.getByRole('button', { name: 'Retry request', exact: true }).click();
        await saved();
        await expect(screen.locator('[aria-invalid="true"]')).toHaveCount(0);
        const corrected = draft().payload;
        if (kind === 'name') assert.deepEqual(corrected.tasks, []);
        else assert.deepEqual(corrected.tasks[0].access, { kind: 'everyone' });
      }
    }
    const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id, org_id FROM users WHERE email = 'admin@example.com') u"));
    const clientId = '01960000-0000-7000-8000-000000000203';
    const taskId = '01960000-0000-7000-8000-000000000204';
    const personId = '01960000-0000-7000-8000-000000000205';
    sql(`INSERT INTO clients (id, org_id, name, currency) VALUES ('${clientId}', '${actor.org_id}', 'Reference recovery client', 'EUR');
      INSERT INTO tasks (id, org_id, name) VALUES ('${taskId}', '${actor.org_id}', 'Reference recovery task');
      INSERT INTO users (id, org_id, email, name, org_role) VALUES ('${personId}', '${actor.org_id}', 'reference-recovery@example.test', 'Reference recovery person', 'member')`);
    const references = [
      { kind: 'client', table: 'clients', id: clientId, control: 'np-client', error: 'np-basic-field-error' },
      { kind: 'task', table: 'tasks', id: taskId, control: `np-task-remove-${rowId}`, error: `np-task-error-${rowId}` },
      { kind: 'person', table: 'users', id: personId, control: `np-person-remove-${personId}`, error: `np-person-error-${personId}` },
      { kind: 'archived-name', table: 'tasks', id: taskId, control: `np-task-remove-${rowId}`, error: `np-task-error-${rowId}` },
    ];
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      for (const reference of references) {
        await readsFinished();
        await page.goto(`${base}/projects`);
        await readsFinished();
        const before = draft();
        const form = { ...before.payload, tasks: [], team: [] };
        if (reference.kind === 'client') form.client_id = clientId;
        if (['task', 'archived-name'].includes(reference.kind)) form.tasks = [{ id: rowId,
          source: reference.kind === 'task' ? { kind: 'existing', task_id: taskId } : { kind: 'new', name: 'Reference recovery task' },
          billable: true, rate: '', budget: '', access: { kind: 'everyone' } }];
        if (reference.kind === 'person') form.team = [{ user_id: personId, manager: false, billable_rate: '', cost_rate: '', budget: '' }];
        const response = await context.request.post(saveRequest.url(), {
          headers: { 'content-type': saveRequest.headers()['content-type'] },
          data: JSON.stringify({ draft_id: draftId, expected_revision: before.revision, form }),
        });
        assert.equal(response.status(), 200, await response.text());
        await page.goto(`${base}/projects/new`);
        await saved();
        await readsFinished();
        const projectsBefore = sql('SELECT count(*) FROM projects');
        sql(`UPDATE ${reference.table} SET active = false WHERE id = '${reference.id}'`);
        try {
          const rejected = page.waitForResponse(response => response.url().includes('/api/finalize_project_draft') && response.status() === (reference.kind === 'archived-name' ? 409 : 404));
          await screen.getByRole('button', { name: 'Save project', exact: true }).focus();
          await page.keyboard.press('Enter');
          await (await rejected).finished();
          const field = screen.locator(`#${reference.control}`), message = screen.locator(`#${reference.error}`);
          await expect(field).toHaveAttribute('aria-invalid', 'true');
          await expect(field).toHaveAttribute('aria-describedby', reference.error);
          await expect(field).toBeFocused();
          await expect(field).toBeEnabled();
          await expect(message).not.toBeEmpty();
          assert.deepEqual(draft().payload, form);
          assert.equal(sql('SELECT count(*) FROM projects'), projectsBefore);
          const footer = await screen.locator('.np-footer').boundingBox();
          for (const item of [field, message]) {
            const box = await item.boundingBox();
            assert.ok(box.y >= 0 && box.y + box.height <= footer.y, `${reference.kind} recovery visible at ${width}`);
          }
          const reloaded = width === 1440 && reference.kind !== 'archived-name';
          if (reloaded) {
            await readsFinished();
            await page.reload();
            await saved();
            if (reference.kind === 'client') {
              await expect(screen).toContainText('This client is archived or unavailable');
              await expect(screen.getByRole('button', { name: 'Save project', exact: true })).toBeDisabled();
            } else {
              await expect(screen).toContainText(reference.kind === 'task' ? 'Unavailable task' : 'Unavailable teammate');
            }
            assert.deepEqual(draft().payload, form, 'Resuming a draft must preserve unavailable selections');
            await field.focus();
          }
          await page.keyboard.press('Enter');
          if (reference.kind === 'client') {
            await page.getByRole('dialog', { name: 'Choose Client', exact: true }).getByRole('option', { name: 'Acme Corp', exact: true }).click();
          } else {
            await expect(screen.locator(reference.kind === 'person' ? '#np-add-person' : '#np-task-search')).toBeFocused();
          }
          if (!reloaded) await screen.getByRole('button', { name: 'Retry request', exact: true }).click();
          await saved();
          await expect(screen.locator('[aria-invalid="true"]')).toHaveCount(0);
          await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Task error recovery project');
          const corrected = draft().payload;
          if (reference.kind === 'client') assert.notEqual(corrected.client_id, clientId);
          else assert.deepEqual(reference.kind === 'person' ? corrected.team : corrected.tasks, []);
        } finally { sql(`UPDATE ${reference.table} SET active = true WHERE id = '${reference.id}'`); }
      }
    }
    console.log('PASS: stale client/task/person references and archived task-name conflicts preserve drafts and support field-level recovery at three widths');
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await page.waitForURL(/\/projects\/[0-9a-f-]{36}$/);
    await expect(page.getByRole('region', { name: 'Project details', exact: true })).toContainText('Task error recovery project');
    await readsFinished();
    assert.deepEqual(errors, []);
    console.log('PASS: saved invalid task names/access identify their row, preserve input and support keyboard recovery at three widths before real finalization');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
