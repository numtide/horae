// All writes belong to run-design-checks.sh's disposable seeded database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const acme = sql("SELECT id FROM clients WHERE name = 'Acme Corp'");
  const tech = sql("SELECT id FROM clients WHERE name = 'TechStart Inc'");
  const projects = sql("SELECT coalesce(jsonb_agg(to_jsonb(p) ORDER BY id), '[]') FROM projects p");
  const invoices = sql("SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY id), '[]') FROM invoices i");
  const draft = () => sql('SELECT row_to_json(d) FROM project_drafts d WHERE discarded_at IS NULL AND completed_project_id IS NULL');
  const editor = page.locator('.np-page');
  const saved = () => expect(editor.locator('header').getByRole('status')).toContainText('Draft saved at');
  const discard = async () => {
    await editor.getByRole('button', { name: 'Discard draft', exact: true }).click();
    await page.getByRole('dialog').getByRole('button', { name: 'Discard draft', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    // Preserve independence when another browser suite left its own test draft.
    if (draft()) {
      await page.goto(`${base}/projects/new`);
      await expect(editor.locator('#np-name')).toBeVisible();
      await discard();
    }
    await page.goto(`${base}/clients/${acme}`);
    await page.getByRole('link', { name: 'View in Projects', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/client/${acme}`);
    await expect(page.getByRole('link', { name: /Acme Website Redesign/ })).toBeVisible();
    await expect(page.getByRole('link', { name: /TechStart API Integration/ })).toHaveCount(0);
    await page.goto(`${base}/clients/${acme}`);
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(editor.locator('#np-client')).toContainText('Acme Corp');
    await expect(editor.locator('#np-currency')).toContainText('Client default');
    await editor.locator('#np-name').fill('Context must preserve this draft');
    await expect.poll(() => draft() && JSON.parse(draft()).payload.name).toBe('Context must preserve this draft');
    await saved();
    const original = draft();
    assert.equal(JSON.parse(original).payload.currency, null);
    for (const client of [tech, 'not-a-client']) {
      await page.goto(`${base}/projects/new/client/${client}`);
      await expect(editor.locator('#np-name')).toHaveValue('Context must preserve this draft');
      await expect(editor.locator('#np-client')).toContainText('Acme Corp');
      await saved();
      assert.equal(draft(), original, 'context does not rewrite a restored draft');
    }
    await editor.locator('#np-client').click();
    await page.getByRole('option', { name: 'Select a client…', exact: true }).click();
    await expect.poll(() => JSON.parse(draft()).payload.client_id).toBe(null);
    await saved();
    const emptyClientDraft = draft();
    await page.goto(`${base}/projects/new/client/${tech}`);
    await expect(editor.locator('#np-client')).toContainText('Select a client');
    await saved();
    assert.equal(draft(), emptyClientDraft, 'an empty draft client is not overwritten');
    await discard();
    for (const client of ['not-a-client', '01950000-0000-7000-8000-ffffffffffff']) {
      await page.goto(`${base}/projects/new/client/${client}`);
      await expect(page.getByRole('alert')).toContainText(/Invalid client|unavailable/);
      await expect(editor).toHaveCount(0);
      assert.equal(draft(), '');
      await page.goto(`${base}/invoices/new/client/${client}`);
      await expect(page.getByRole('alert')).toContainText(/Invalid client|unavailable/);
      await expect(page.locator('#inv-client')).toHaveCount(0);
    }
    await page.goto(`${base}/clients/${tech}`);
    await page.getByRole('link', { name: 'New invoice', exact: true }).click();
    await expect(page.locator('#inv-client')).toHaveValue(tech);
    await page.locator('#inv-client').selectOption(acme);
    await page.locator('#inv-from').fill('2026-09-01');
    await expect(page.locator('#inv-client')).toHaveValue(acme);
    assert.equal(sql("SELECT coalesce(jsonb_agg(to_jsonb(p) ORDER BY id), '[]') FROM projects p"), projects);
    assert.equal(sql("SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY id), '[]') FROM invoices i"), invoices);
    assert.deepEqual(errors, []);
    console.log('PASS: client links filter real projects, prefill once, preserve every draft and never create business records');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exit(1); });
