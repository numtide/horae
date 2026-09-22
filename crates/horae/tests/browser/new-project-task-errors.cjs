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
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await page.waitForURL(/\/projects\/[0-9a-f-]{36}$/);
    await expect(page.getByRole('region', { name: 'Project details', exact: true })).toContainText('Task error recovery project');
    await readsFinished();
    assert.deepEqual(errors, []);
    console.log('PASS: saved invalid task names/access identify their row, preserve input and support keyboard recovery at three widths before real finalization');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
