// Fixtures are restricted to the browser runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
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
const branding = column => sql(`SELECT coalesce(${column}, '<null>') FROM organizations WHERE id='${org}'`);
const columns = ['provider_name', 'provider_address', 'provider_tax_id', 'provider_email', 'provider_phone',
  'bank_name', 'bank_iban', 'bank_bic', 'bank_routing', 'bank_account', 'invoice_notes', 'invoice_payment_terms'];
const before = JSON.parse(sql(`SELECT row_to_json(o) FROM (SELECT ${columns.join(',')}, permission_policy_version FROM organizations WHERE id='${org}') o`));
assert.equal(before.permission_policy_version, 0);
const literal = value => value === null ? 'NULL' : `'${String(value).replaceAll("'", "''")}'`;

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const main = page.locator('main');
  const settingsLink = main.getByRole('link', { name: 'Settings', exact: true });
  const asRole = async role => {
    await page.goto(`${base}/invoices`);
    await expect(page.locator('.sidebar-user-sub')).toHaveText(role[0].toUpperCase() + role.slice(1));
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    await asRole('admin');
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

    // Members neither see the action nor get the form from a direct link.
    sql(`UPDATE users SET org_role='member' WHERE id='${actor.id}'`);
    await asRole('member');
    await expect(settingsLink).toHaveCount(0);
    await page.goto(`${base}/invoices/settings`);
    await expect(main.getByRole('alert')).toContainText('managers and administrators');
    await expect(page.getByLabel('Company name', { exact: true })).toHaveCount(0);
    await expect(main).not.toContainText('Acme Studio');
    console.log('PASS: members are not offered invoice settings and a direct link shows no values');

    // Detailed permissions leave the decision to the role, as the server does.
    sql(`BEGIN; UPDATE users SET org_role='manager' WHERE id='${actor.id}';
      INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ('019f4300-0000-7000-8000-000000000001','${org}','${actor.id}',1,
          ARRAY['time_read_own','time_write_own','expense_read_own','expense_write_own'],false,'individual');
      UPDATE organizations SET permission_policy_version=1 WHERE id='${org}'; COMMIT;`);
    // Under policy one the sidebar names no legacy role, so the action itself is the signal.
    await page.goto(`${base}/invoices`);
    await settingsLink.click();
    await expect(page.getByLabel('Company name', { exact: true })).toHaveValue('Acme Studio S.L.');
    console.log('PASS: managers open invoice settings under detailed permissions');

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
    sql(`BEGIN; UPDATE users SET org_role='admin' WHERE id='${actor.id}';
      DELETE FROM person_permission_states WHERE user_id='${actor.id}';
      UPDATE organizations SET permission_policy_version=0,
        ${columns.map(column => `${column}=${literal(before[column])}`).join(', ')}
        WHERE id='${org}'; COMMIT;`);
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
