// Invoice settings on stored grants. Fixtures are restricted to the browser runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const actor = JSON.parse(sql("SELECT row_to_json(u) FROM (SELECT id,org_id,org_role FROM users WHERE email='admin@example.com' AND active) u"));
const org = actor.org_id;
const state = '019f4300-0000-7000-8000-000000000001';
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];
const writer = [...floor, 'invoice_read_managed', 'invoice_draft_write_managed', 'invoice_write_managed',
  'invoice_read_all', 'invoice_write_all'];
const reader = [...floor, 'invoice_read_managed', 'invoice_read_all'];
const grantsSql = grants => `ARRAY[${grants.map(grant => `'${grant}'`).join(',')}]`;
const setGrants = grants => sql(`UPDATE person_permission_states SET grants=${grantsSql(grants)} WHERE id='${state}'`);
const branding = column => sql(`SELECT coalesce(${column}, '<null>') FROM organizations WHERE id='${org}'`);
const columns = ['provider_name', 'provider_address', 'provider_tax_id', 'provider_email', 'provider_phone',
  'bank_name', 'bank_iban', 'bank_bic', 'bank_routing', 'bank_account', 'invoice_notes', 'invoice_payment_terms'];
const before = JSON.parse(sql(`SELECT row_to_json(o) FROM (SELECT ${columns.join(',')}, permission_policy_version FROM organizations WHERE id='${org}') o`));
const literal = value => value === null ? 'NULL' : `'${String(value).replaceAll("'", "''")}'`;
// Only this suite's state row is created and removed; the policy and role are restored to their observed values.
assert.equal(sql(`SELECT count(*) FROM person_permission_states WHERE user_id='${actor.id}'`), '0');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const main = page.locator('main');
  const settingsLink = main.getByRole('link', { name: 'Settings', exact: true });
  // The action follows the session's permissions, so wait for them to load.
  const invoices = async () => {
    const permissions = page.waitForResponse(r => r.url().includes('/api/get_my_permissions') && r.status() === 200);
    await page.goto(`${base}/invoices`);
    await permissions;
    await expect(page.getByRole('heading', { name: 'Invoices', level: 1 })).toBeVisible();
  };
  try {
    // The page shows grants only once the organization answers from stored state.
    sql(`BEGIN;
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('${state}','${org}','${actor.id}',1,${grantsSql(writer)},false,'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    await invoices();
    await settingsLink.click();
    await expect(page).toHaveURL(`${base}/invoices/settings`);
    await expect(page.getByRole('heading', { name: 'Invoice settings', level: 1 })).toBeVisible();
    await expect(page.locator('.nav-item.active')).toHaveText(/Invoices/);
    await expect(page.getByLabel('Payment terms', { exact: true })).toHaveValue(before.invoice_payment_terms ?? '');

    // One save per card: the company save leaves an unsaved bank edit alone.
    const company = page.locator('form', { has: page.getByRole('heading', { name: 'Company information' }) });
    await page.getByLabel('Company name', { exact: true }).fill('Acme Studio S.L.');
    await page.getByLabel('Address', { exact: true }).fill('Calle Mayor 1\n28013 Madrid');
    await page.getByLabel('Tax ID', { exact: true }).fill('ESB12345678');
    await page.getByLabel('Bank name', { exact: true }).fill('Unsaved bank');
    await company.getByRole('button', { name: 'Save company information' }).click();
    await expect(company.getByRole('status')).toHaveText('Saved.');
    assert.equal(branding('provider_name'), 'Acme Studio S.L.');
    assert.equal(branding('provider_address'), 'Calle Mayor 1\n28013 Madrid');
    assert.equal(branding('bank_name'), before.bank_name ?? '<null>');
    await expect(page.getByLabel('Bank name', { exact: true })).toHaveValue('Unsaved bank');
    const preview = page.getByRole('region', { name: 'Preview' });
    await expect(preview).toContainText('Acme Studio S.L.');
    await expect(preview).toContainText('Tax ID: ESB12345678');
    await expect(preview).toContainText('Bank: Unsaved bank');
    console.log('PASS: the settings action opens one save per card with a live preview');

    // Inline validation keeps the typed value and saves nothing.
    const payment = page.locator('form', { has: page.getByRole('heading', { name: 'Payment details' }) });
    const iban = page.getByLabel('IBAN', { exact: true });
    await iban.fill('X'.repeat(32768));
    await payment.getByRole('button', { name: 'Save payment details' }).click();
    await expect(iban).toHaveAttribute('aria-invalid', 'true');
    await expect(page.locator('#inv-settings-iban-error')).toContainText('at most 32,767 bytes');
    assert.equal(branding('bank_iban'), before.bank_iban ?? '<null>');
    await iban.fill('ES91 2100 0418 4502 0005 1332');
    await expect(iban).not.toHaveAttribute('aria-invalid', 'true');
    await payment.getByRole('button', { name: 'Save payment details' }).click();
    await expect(payment.getByRole('status')).toHaveText('Saved.');
    assert.equal(branding('bank_iban'), 'ES91 2100 0418 4502 0005 1332');
    assert.equal(branding('bank_name'), 'Unsaved bank');

    const defaults = page.locator('form', { has: page.getByRole('heading', { name: 'Default values' }) });
    await page.getByLabel('Payment terms', { exact: true }).fill('');
    await page.getByLabel('Invoice notes', { exact: true }).fill('Thank you for your business.');
    await defaults.getByRole('button', { name: 'Save default values' }).click();
    await expect(defaults.getByRole('status')).toHaveText('Saved.');
    assert.equal(branding('invoice_payment_terms'), '<null>');
    assert.equal(branding('invoice_notes'), 'Thank you for your business.');
    await page.reload();
    await expect(page.getByLabel('Invoice notes', { exact: true })).toHaveValue('Thank you for your business.');
    await expect(page.getByLabel('Company name', { exact: true })).toHaveValue('Acme Studio S.L.');
    console.log('PASS: inline validation, blank values cleared and saved values survive a reload');

    // Someone else saves the IBAN while this page is open: a later save here must not restore it.
    sql(`UPDATE organizations SET bank_iban='DE89 3704 0044 0532 0130 00' WHERE id='${org}'`);
    await page.getByLabel('Phone', { exact: true }).fill('+34 600 000 000');
    await company.getByRole('button', { name: 'Save company information' }).click();
    await expect(company.getByRole('alert')).toContainText('Someone changed these settings');
    await expect(company.getByRole('alert')).not.toContainText('error running server function');
    await expect(page.getByLabel('Phone', { exact: true })).toHaveValue('+34 600 000 000');
    assert.equal(branding('bank_iban'), 'DE89 3704 0044 0532 0130 00');
    assert.equal(branding('provider_phone'), before.provider_phone ?? '<null>');
    await page.reload();
    await expect(page.getByLabel('IBAN', { exact: true })).toHaveValue('DE89 3704 0044 0532 0130 00');
    console.log('PASS: a save from a stale page is refused and keeps the newer values');

    // Reading invoices is not enough: no action, and a direct link shows no values.
    setGrants(reader);
    await invoices();
    await expect(settingsLink).toHaveCount(0);
    await page.goto(`${base}/invoices/settings`);
    await expect(main.getByRole('alert')).toContainText('people who manage all invoices');
    await expect(page.getByLabel('Company name', { exact: true })).toHaveCount(0);
    await expect(main).not.toContainText('Acme Studio');
    console.log('PASS: invoice readers are not offered invoice settings and a direct link shows no values');

    // The dead legacy role decides nothing: a member with the grant opens the settings.
    sql(`UPDATE users SET org_role='member' WHERE id='${actor.id}'`);
    setGrants(writer);
    await invoices();
    await settingsLink.click();
    await expect(page.getByLabel('Company name', { exact: true })).toHaveValue('Acme Studio S.L.');
    console.log('PASS: the invoice write grant opens invoice settings whatever the legacy role');

    // 390px: the cards and preview stack without horizontal scrolling.
    await page.setViewportSize({ width: 390, height: 844 });
    await page.reload();
    await expect(page.getByLabel('Company name', { exact: true })).toBeVisible();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
    assert.deepEqual(errors, []);
    console.log('PASS: invoice settings fit a 390px viewport');
  } catch (error) {
    console.error(error);
    console.error({ url: page.url(), errors, page: await page.locator('body').ariaSnapshot({ timeout: 3000 }).catch(failure => failure.message) });
    throw error;
  } finally {
    sql(`BEGIN; UPDATE users SET org_role=${literal(actor.org_role)} WHERE id='${actor.id}';
      DELETE FROM person_permission_states WHERE id='${state}';
      UPDATE organizations SET permission_policy_version=${before.permission_policy_version},
        ${columns.map(column => `${column}=${literal(before[column])}`).join(', ')}
        WHERE id='${org}'; COMMIT;`);
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
