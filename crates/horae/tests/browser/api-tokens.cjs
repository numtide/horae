// Run only through run-design-checks.sh: it creates and revokes tokens of the
// dev-login administrator in the runner's disposable database.
// The tokens table: name first in bold with the token prefix under it, mono
// dates, status badges and one «⋮» menu whose "Revoke…" asks first.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(['127.0.0.1', 'localhost'].includes(new URL(base).hostname), 'Fixtures require a loopback instance');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  const name = `Browser check ${Date.now()}`;
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);

    await page.goto(`${base}/settings/api-tokens`);
    await expect(page.locator('#api-token-name')).toHaveAttribute('data-dioxus-id', /\d+/);
    await page.locator('#api-token-name').fill(name);
    await page.locator('#api-token-create').click();
    await page.locator('#api-token-dismiss').click();

    const row = page.locator('tbody tr', { hasText: name });
    const first = row.locator('td').first();
    await expect(first.locator('.name-cell-name')).toHaveText(name);
    await expect(first.locator('.name-cell-secondary')).toContainText('horae_pat_');
    await expect(row.locator('td').nth(1)).toHaveCSS('font-family', /mono/i);
    await expect(row.locator('.badge-success')).toHaveText('Active');
    await expect(row.getByRole('button', { name: 'Revoke', exact: true })).toHaveCount(0);
    // On a desktop the whole table, its row menus included, fits the frame.
    assert.ok(await page.locator('.table-container').evaluate(frame => frame.scrollWidth <= frame.clientWidth + 1),
      'the tokens table does not scroll sideways at 1440px');

    // The one action is in the row's menu, named after the token.
    await row.getByRole('button', { name: `Actions for ${name}`, exact: true }).click();
    const revoke = page.getByRole('menuitem', { name: 'Revoke…', exact: true });
    await expect(revoke).toBeVisible();
    await expect(revoke).toHaveClass(/danger/);
    await revoke.click();
    const dialog = page.getByRole('dialog', { name: `Revoke “${name}”?` });
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Revoke', exact: true }).click();
    await expect(row.locator('.badge')).toHaveText('Revoked');
    // A revoked token has no actions left, so no menu.
    await expect(row.getByRole('button', { name: `Actions for ${name}` })).toHaveCount(0);
    await expect(page.locator('#api-tokens-title')).toBeFocused();

    // The workspace list shows the owner after the name and the same states.
    await page.goto(`${base}/admin/api-tokens`);
    const workspace = page.getByRole('region', { name: 'Workspace tokens', exact: true });
    await expect(workspace.locator('tbody tr', { hasText: name }).locator('.badge')).toHaveText('Revoked');
    const columns = await workspace.locator('thead th').allTextContents();
    assert.deepEqual(columns.slice(0, 2).map(c => c.trim().toLowerCase()), ['name', 'owner']);

    assert.deepEqual(errors, [], 'No browser runtime errors');
    console.log('PASS: API tokens table, menu revoke and workspace list');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
