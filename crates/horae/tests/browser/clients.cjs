// Only run through run-design-checks.sh against its disposable seeded database.
const { chromium, expect: baseExpect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
// Client reads may still be pending when navigation or dialog dismissal finishes.
const expect = baseExpect.configure({ timeout: 15000 });
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to an isolated test instance');
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const name = `Clients browser ${Date.now()}`;
  const dialog = page.getByRole('dialog');
  const field = label => dialog.getByRole('textbox', { name: label, exact: true });
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/clients`);
    const create = page.getByRole('button', { name: 'New client', exact: true }).first();
    await create.click();
    await expect(dialog).toBeVisible();
    await expect(field('Client name')).toBeFocused();
    await dialog.getByRole('button', { name: 'Create client', exact: true }).click();
    await expect(dialog.getByRole('alert')).toContainText('1–200 characters');
    await expect(dialog.getByRole('alert')).toBeFocused();
    await field('Client name').fill(name);
    const dirtyNavigation = page.waitForEvent('dialog');
    const navigation = page.evaluate(() => history.pushState(null, '', '/projects'));
    const warning = await dirtyNavigation;
    assert.equal(warning.message(), 'Discard unsaved client changes?');
    await warning.dismiss();
    await navigation;
    await expect(page).toHaveURL(`${base}/clients`);
    await expect(field('Client name')).toHaveValue(name);
    await field('Default rate').fill('0');
    await field('Billing address').fill('Test street\nFloor 2');
    await field('Tax ID (optional)').fill('TEST-VAT');
    await dialog.getByRole('combobox', { name: 'Currency', exact: true }).selectOption('EUR');
    await dialog.getByRole('button', { name: 'Create client', exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(create).toBeFocused();
    await page.getByRole('searchbox', { name: 'Search clients by name' }).fill(name);
    const link = page.getByRole('link', { name, exact: true });
    await expect(link).toHaveCount(1);
    const rowEdit = page.getByRole('row').filter({ has: link }).getByRole('button', { name: 'Edit', exact: true });
    await rowEdit.click();
    await expect(field('Tax ID (optional)')).toHaveValue('TEST-VAT');
    await field('Tax ID (optional)').fill('TEST-VAT-2');
    await dialog.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(rowEdit).toBeFocused();
    const detailPath = await link.getAttribute('href');
    assert.match(detailPath, /^\/clients\/[0-9a-f-]+$/);
    await page.reload();
    await page.getByRole('link', { name, exact: true }).click();
    await expect(page.getByRole('heading', { name, exact: true })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Billing', exact: true })).toContainText('EUR 0.00 / h');
    const edit = page.getByRole('button', { name: 'Edit client', exact: true });
    await edit.click();
    await expect(field('Default rate')).toHaveValue('0.00');
    await expect(field('Billing address')).toHaveValue('Test street\nFloor 2');
    await field('Billing address').fill('Must not be saved');
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(edit).toBeFocused();
    await page.reload();
    await expect(page.getByRole('region', { name: 'Billing', exact: true })).toContainText('Test street');
    await edit.click();
    await dialog.getByRole('combobox', { name: 'Currency', exact: true }).selectOption('USD');
    await dialog.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(dialog.getByRole('alert')).toContainText('Confirm the default rate');
    await expect(field('Default rate')).toHaveValue('0.00');
    await dialog.getByRole('button', { name: 'Use this rate in USD', exact: true }).click();
    await dialog.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(edit).toBeFocused();
    await expect(page.getByRole('region', { name: 'Billing', exact: true })).toContainText('USD 0.00 / h');
    await page.reload();
    await expect(page.getByRole('region', { name: 'Billing', exact: true })).toContainText('USD 0.00 / h');
    console.log('PASS: clients create/reload/edit/cancel and explicit currency/rate intent');

    await page.goto(`${base}/clients`);
    await create.click();
    const uncertainName = `${name} uncertain`;
    await field('Client name').fill(uncertainName);
    let release;
    let requests = 0;
    const gate = new Promise(resolve => { release = resolve; });
    const route = url => url.pathname.includes('/api/create_client_profile');
    await page.route(route, async request => { requests++; await gate; await request.abort('failed'); });
    try {
      const sent = page.waitForRequest(request => request.url().includes('/api/create_client_profile'));
      await dialog.getByRole('button', { name: 'Create client', exact: true }).click();
      await sent;
      await expect(dialog.getByRole('button', { name: 'Saving…', exact: true })).toBeDisabled();
      await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
      await dialog.locator('form').evaluate(form => form.requestSubmit());
      await page.keyboard.press('Escape');
      await page.mouse.click(4, 4);
      await expect(dialog).toBeVisible();
      assert.equal(requests, 1, 'pending submission is not duplicated');
      const pendingNavigation = page.waitForEvent('dialog');
      const navigation = page.evaluate(() => history.pushState(null, '', '/projects'));
      const warning = await pendingNavigation;
      assert.equal(warning.message(), 'A client save is pending. Wait for it to finish before leaving.');
      await warning.accept();
      await navigation;
      await expect(page).toHaveURL(`${base}/clients`);
      await expect(field('Client name')).toHaveValue(uncertainName);
    } finally { release(); }
    await expect(dialog.getByRole('status')).toContainText('Creation may have completed');
    await expect(field('Client name')).toHaveValue(uncertainName);
    await expect(dialog.getByRole('button', { name: 'Create client', exact: true })).toBeDisabled();
    await dialog.locator('form').evaluate(form => form.requestSubmit());
    assert.equal(requests, 1, 'uncertain creation is not automatically retried');
    const popupReady = page.waitForEvent('popup');
    await dialog.getByRole('link', { name: 'Check client list in a new tab', exact: true }).click();
    const popup = await popupReady;
    await expect(popup.getByRole('heading', { name: 'Clients', exact: true })).toBeVisible();
    await popup.close();
    await expect(field('Client name')).toHaveValue(uncertainName);
    await page.unroute(route);
    await dialog.getByRole('button', { name: 'I checked the list — allow another attempt', exact: true }).click();
    await dialog.getByRole('button', { name: 'Create client', exact: true }).click();
    await expect(dialog).toBeHidden();
    await page.getByRole('searchbox', { name: 'Search clients by name' }).fill(uncertainName);
    await expect(page.getByRole('link', { name: uncertainName, exact: true })).toHaveCount(1);
    console.log('PASS: pending guard and explicit uncertain-create recovery retain input');

    await create.click();
    const committedName = `${name} committed`;
    await field('Client name').fill(committedName);
    let committedRequests = 0;
    await page.route(route, async intercepted => {
      committedRequests++;
      const response = await intercepted.fetch();
      assert.ok(response.ok(), 'the server accepted creation before its response was lost');
      await intercepted.abort('failed');
    });
    await dialog.getByRole('button', { name: 'Create client', exact: true }).click();
    await expect(dialog.getByRole('status')).toContainText('Creation may have completed');
    await expect(field('Client name')).toHaveValue(committedName);
    const committedPopupReady = page.waitForEvent('popup');
    await dialog.getByRole('link', { name: 'Check client list in a new tab', exact: true }).click();
    const committedPopup = await committedPopupReady;
    await committedPopup.getByRole('searchbox', { name: 'Search clients by name' }).fill(committedName);
    await expect(committedPopup.getByRole('link', { name: committedName, exact: true })).toHaveCount(1);
    await committedPopup.close();
    await dialog.locator('form').evaluate(form => form.requestSubmit());
    await expect(dialog.getByRole('button', { name: 'Create client', exact: true })).toBeDisabled();
    assert.equal(committedRequests, 1, 'lost acknowledgement does not duplicate the committed client');
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.unroute(route);
    await page.reload();
    await page.getByRole('searchbox', { name: 'Search clients by name' }).fill(committedName);
    await expect(page.getByRole('link', { name: committedName, exact: true })).toHaveCount(1);
    console.log('PASS: lost creation acknowledgement can be inspected without duplicating or undoing the client');

    // Archive needs every project archived first; restore changes only the client.
    const history = () => sql(`SELECT jsonb_build_object(
      'projects', (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM projects t),
      'entries', (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM time_entries t),
      'invoices', (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM invoices t))`);
    const before = history();
    const clientActive = id => sql(`SELECT active FROM clients WHERE id = '${id}'`);
    let statusRequests = 0;
    page.on('request', request => { if (request.url().includes('/api/set_client_active')) statusRequests++; });
    const search = page.getByRole('searchbox', { name: 'Search clients by name' });
    const rowFor = client => page.getByRole('row').filter({ has: page.getByRole('link', { name: client, exact: true }) });
    const scopeTo = async label => {
      await page.locator('#client-scope-menu-trigger').click();
      await page.getByRole('menuitem', { name: new RegExp(`^${label}`) }).click();
    };
    const archiveDialog = client => page.getByRole('dialog', { name: `Archive ${client}?` });

    await search.fill('Acme Corp');
    const acme = rowFor('Acme Corp');
    const acmeId = sql("SELECT id FROM clients WHERE name = 'Acme Corp'");
    const acmeArchive = acme.getByRole('button', { name: 'Archive', exact: true });
    for (const dismiss of ['Cancel', 'Escape']) {
      await acmeArchive.click();
      await expect(archiveDialog('Acme Corp')).toBeVisible();
      await expect(archiveDialog('Acme Corp')).toContainText('Only the client is archived. Its projects must already be archived.');
      await expect(archiveDialog('Acme Corp')).not.toContainText(/\d+\s+projects?/);
      if (dismiss === 'Cancel') await archiveDialog('Acme Corp').getByRole('button', { name: 'Cancel', exact: true }).click();
      else await page.keyboard.press('Escape');
      await expect(archiveDialog('Acme Corp')).toBeHidden();
      await expect(acmeArchive).toBeFocused();
    }
    assert.equal(statusRequests, 0, 'Cancel and Escape send nothing');
    await acmeArchive.click();
    await archiveDialog('Acme Corp').getByRole('button', { name: 'Archive client', exact: true }).click();
    await expect(archiveDialog('Acme Corp').getByRole('alert')).toContainText("Archive this client's projects before archiving the client.");
    await expect(archiveDialog('Acme Corp').getByRole('alert')).not.toContainText(/\d/);
    console.log(`OBSERVE: focus after a refused archive is ${await page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName)}`);
    assert.equal(
      await archiveDialog('Acme Corp').getByRole('link', { name: "View this client's projects", exact: true }).getAttribute('href'),
      `/projects/client/${acmeId}`,
    );
    await archiveDialog('Acme Corp').getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(acme.getByText('Active', { exact: true })).toBeVisible();
    assert.equal(clientActive(acmeId), 't');
    assert.equal(history(), before);
    console.log('PASS: archive confirms without project counts, Cancel/Escape change nothing, active projects refuse with a recovery link');

    // Narrow screens with enlarged text: labels keep their words, nothing clips
    // vertically, and both actions stay reachable by scrolling and keyboard.
    const originalTheme = await page.evaluate(() => document.documentElement.dataset.theme ?? null);
    for (const theme of ['light', 'dark']) {
      for (const [width, height] of [[320, 900], [390, 844]]) {
        const label = `${theme}/${width}x${height}/text 200%`;
        await page.setViewportSize({ width, height });
        await page.evaluate(theme => {
          document.documentElement.dataset.theme = theme;
          document.documentElement.style.fontSize = '200%';
        }, theme);
        await acmeArchive.scrollIntoViewIfNeeded();
        await acmeArchive.click();
        await expect(archiveDialog('Acme Corp')).toBeVisible();
        for (const name of ['Archive client', 'Cancel']) {
          const action = archiveDialog('Acme Corp').getByRole('button', { name, exact: true });
          await action.scrollIntoViewIfNeeded();
          await expect(action).toBeInViewport({ ratio: 1 });
          const layout = await action.evaluate(button => {
            const range = document.createRange();
            range.selectNodeContents(button);
            const lines = new Set([...range.getClientRects()].map(rect => Math.round(rect.top))).size;
            return { lines, clipped: button.scrollHeight > button.clientHeight + 1 || button.scrollWidth > button.clientWidth + 1 };
          });
          // At most one line per word: a label may wrap between words, never inside one.
          assert.ok(layout.lines <= name.split(' ').length, `${label}: "${name}" wraps into ${layout.lines} lines`);
          assert.equal(layout.clipped, false, `${label}: "${name}" clips its label`);
        }
        await archiveDialog('Acme Corp').getByRole('button', { name: 'Archive client', exact: true }).focus();
        await page.keyboard.press('Tab');
        await expect(archiveDialog('Acme Corp').getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
        await expect(archiveDialog('Acme Corp').getByRole('button', { name: 'Cancel', exact: true })).toBeInViewport({ ratio: 1 });
        await page.keyboard.press('Escape');
        await expect(archiveDialog('Acme Corp')).toBeHidden();
      }
    }
    await page.evaluate(theme => {
      document.documentElement.style.fontSize = '';
      if (theme === null) delete document.documentElement.dataset.theme;
      else document.documentElement.dataset.theme = theme;
    }, originalTheme);
    await page.setViewportSize({ width: 1440, height: 900 });
    assert.equal(statusRequests, 1, 'layout checks send nothing');
    console.log('PASS: the archive confirmation keeps readable, reachable actions at 320 and 390px with 200% text in both themes');

    // A client without projects: a pending archive cannot be repeated, and the
    // row leaving the Active filter returns focus to the page heading.
    await search.fill(committedName);
    const committed = rowFor(committedName);
    const committedId = sql(`SELECT id FROM clients WHERE name = '${committedName.replaceAll("'", "''")}'`);
    let releaseArchive;
    const held = new Promise(resolve => { releaseArchive = resolve; });
    await page.route('**/api/set_client_active*', async route => { await held; await route.continue(); });
    statusRequests = 0;
    await committed.getByRole('button', { name: 'Archive', exact: true }).click();
    const confirm = archiveDialog(committedName).getByRole('button', { name: /^Archiv/ });
    await confirm.click();
    await expect(confirm).toBeDisabled();
    await expect(archiveDialog(committedName)).toHaveAttribute('aria-busy', 'true');
    await page.keyboard.press('Escape');
    await expect(archiveDialog(committedName)).toBeVisible();
    await confirm.click({ force: true });
    releaseArchive();
    await expect(archiveDialog(committedName)).toBeHidden();
    await expect(committed).toHaveCount(0);
    await expect(page.locator('#clients-heading')).toBeFocused();
    await page.unroute('**/api/set_client_active*');
    assert.equal(statusRequests, 1, 'a pending archive is sent once');
    assert.equal(clientActive(committedId), 'f');
    assert.equal(history(), before);

    await expect(page.getByRole('status').filter({ hasText: `${committedName} archived` })).toBeVisible();

    // Restore is direct, shows that it is pending and sends once. A row that
    // stays visible keeps focus on its control.
    await scopeTo('All clients');
    await expect(committed.getByText('Archived', { exact: true })).toBeVisible();
    let releaseRestore;
    const heldRestore = new Promise(resolve => { releaseRestore = resolve; });
    await page.route('**/api/set_client_active*', async route => { await heldRestore; await route.continue(); });
    statusRequests = 0;
    await committed.getByRole('button', { name: 'Restore', exact: true }).click();
    const restoringButton = committed.getByRole('button', { name: 'Restoring…', exact: true });
    await expect(restoringButton).toBeDisabled();
    await expect(restoringButton).toHaveAttribute('aria-busy', 'true');
    await restoringButton.click({ force: true });
    releaseRestore();
    await expect(committed.getByText('Active', { exact: true })).toBeVisible();
    await page.unroute('**/api/set_client_active*');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(committed.getByRole('button', { name: 'Archive', exact: true })).toBeFocused();
    await expect(page.getByRole('status').filter({ hasText: `${committedName} restored` })).toBeVisible();
    assert.equal(statusRequests, 1, 'a pending restore is sent once');
    assert.equal(clientActive(committedId), 't');
    assert.equal(history(), before);

    // Focus the user moves while the reload is still pending stays there.
    await committed.getByRole('button', { name: 'Archive', exact: true }).click();
    await archiveDialog(committedName).getByRole('button', { name: 'Archive client', exact: true }).click();
    await expect(committed.getByText('Archived', { exact: true })).toBeVisible();
    let releaseReload;
    const heldReload = new Promise(resolve => { releaseReload = resolve; });
    await page.route('**/api/client_overview*', async route => { await heldReload; await route.continue(); }, { times: 1 });
    const reloadStarted = page.waitForRequest(request => request.url().includes('/api/client_overview'));
    await committed.getByRole('button', { name: 'Restore', exact: true }).click();
    await reloadStarted;
    await search.focus();
    releaseReload();
    await expect(committed.getByText('Active', { exact: true })).toBeVisible();
    await expect(search).toBeFocused();

    // A failed restore keeps its alert; where focus lands is observed, not changed.
    await committed.getByRole('button', { name: 'Archive', exact: true }).click();
    await archiveDialog(committedName).getByRole('button', { name: 'Archive client', exact: true }).click();
    await expect(committed.getByText('Archived', { exact: true })).toBeVisible();
    await page.route('**/api/set_client_active*', route => route.abort('failed'), { times: 1 });
    await committed.getByRole('button', { name: 'Restore', exact: true }).click();
    await expect(page.getByRole('alert').filter({ hasText: 'Could not change client status' })).toBeVisible();
    await expect(committed.getByRole('button', { name: 'Restore', exact: true })).toBeEnabled();
    console.log(`OBSERVE: focus after a failed list restore is ${await page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName)}`);
    assert.equal(clientActive(committedId), 'f');

    // Busy overlap: another row's restore finishing reloads the list while an
    // archive is pending. The pending confirmation stays open, and its late
    // refusal appears there and never in a different client's confirmation.
    const gates = [];
    const hold = () => { let release; gates.push(new Promise(resolve => { release = resolve; })); return release; };
    await page.route('**/api/set_client_active*', async route => { await gates.shift(); await route.continue(); });
    const releaseRestoreX = hold(), releaseArchiveY = hold();
    await committed.getByRole('button', { name: 'Restore', exact: true }).click();
    await search.fill('Acme Corp');
    await acmeArchive.click();
    const acmeConfirm = archiveDialog('Acme Corp').getByRole('button', { name: /^Archiv/ });
    await acmeConfirm.click();
    await expect(acmeConfirm).toHaveText('Archiving client…');
    const ordinaryReload = page.waitForResponse(response => response.url().includes('/api/client_overview'));
    releaseRestoreX();
    await ordinaryReload;
    await expect(archiveDialog('Acme Corp')).toBeVisible();
    await expect(acmeConfirm).toBeDisabled();
    const refusal = page.waitForResponse(response => response.url().includes('/api/set_client_active'));
    releaseArchiveY();
    assert.equal((await refusal).status(), 409);
    await expect(archiveDialog('Acme Corp').getByRole('alert')).toContainText("Archive this client's projects before archiving the client.");
    await archiveDialog('Acme Corp').getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.unroute('**/api/set_client_active*');
    assert.equal(clientActive(committedId), 't');
    assert.equal(clientActive(acmeId), 't');
    // A different target opens with no error from the previous one.
    await search.fill(committedName);
    await committed.getByRole('button', { name: 'Archive', exact: true }).click();
    await expect(archiveDialog(committedName)).toBeVisible();
    await expect(archiveDialog(committedName).getByRole('alert')).toHaveCount(0);
    await archiveDialog(committedName).getByRole('button', { name: 'Archive client', exact: true }).click();
    await expect(committed.getByText('Archived', { exact: true })).toBeVisible();

    // Busy, then denied: a settled reload under changed policy clears even a
    // pending confirmation, its late error is dropped, and reopening the same
    // client once authority returns shows no earlier error.
    const busyGates = [];
    const busyHold = () => { let release; busyGates.push(new Promise(resolve => { release = resolve; })); return release; };
    await page.route('**/api/set_client_active*', async route => { await busyGates.shift(); await route.continue(); });
    const releaseBusyRestore = busyHold(), releaseBusyArchive = busyHold();
    let releaseBusyReload;
    const heldBusyReload = new Promise(resolve => { releaseBusyReload = resolve; });
    await committed.getByRole('button', { name: 'Restore', exact: true }).click();
    await search.fill('Acme Corp');
    await acmeArchive.click();
    await acmeConfirm.click();
    await expect(acmeConfirm).toHaveText('Archiving client…');
    await page.route('**/api/client_overview*', async route => { await heldBusyReload; await route.continue(); }, { times: 1 });
    const busyReload = page.waitForRequest(request => request.url().includes('/api/client_overview'));
    releaseBusyRestore();
    await busyReload;
    await expect(archiveDialog('Acme Corp')).toBeVisible();
    const busyOrg = sql(`SELECT org_id FROM clients WHERE id = '${acmeId}'`);
    sql(`UPDATE organizations SET permission_policy_version = 1 WHERE id = '${busyOrg}'`);
    try {
      releaseBusyReload();
      await expect(page.getByRole('alert').filter({ hasText: 'Could not load clients' })).toBeVisible();
      await expect(archiveDialog('Acme Corp')).toBeHidden();
    } finally {
      sql(`UPDATE organizations SET permission_policy_version = 0 WHERE id = '${busyOrg}'`);
    }
    // Authority returns and the list is retried while the old archive is still
    // outstanding: no Archive opener works, for that client or any other, and
    // no confirmation appears for a new target.
    await page.locator('#clients-retry').click();
    await expect(acmeArchive).toBeDisabled();
    await acmeArchive.click({ force: true });
    await search.fill(committedName);
    await expect(committed.getByRole('button', { name: 'Archive', exact: true })).toBeDisabled();
    await committed.getByRole('button', { name: 'Archive', exact: true }).click({ force: true });
    await expect(page.getByRole('dialog')).toHaveCount(0);
    // The old archive then settles; its late refusal reaches no confirmation.
    const lateRefusal = page.waitForResponse(response => response.url().includes('/api/set_client_active'));
    releaseBusyArchive();
    assert.equal((await lateRefusal).status(), 409);
    await page.unroute('**/api/set_client_active*');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(committed.getByRole('button', { name: 'Archive', exact: true })).toBeEnabled();
    await search.fill('Acme Corp');
    await expect(acmeArchive).toBeEnabled();
    await acmeArchive.click();
    await expect(archiveDialog('Acme Corp')).toBeVisible();
    await expect(archiveDialog('Acme Corp').getByRole('alert')).toHaveCount(0);
    await archiveDialog('Acme Corp').getByRole('button', { name: 'Cancel', exact: true }).click();
    assert.equal(clientActive(acmeId), 't');
    await search.fill(committedName);
    await committed.getByRole('button', { name: 'Archive', exact: true }).click();
    await archiveDialog(committedName).getByRole('button', { name: 'Archive client', exact: true }).click();
    await expect(committed.getByText('Archived', { exact: true })).toBeVisible();

    // A policy change during an overlapping reload clears the open
    // confirmation and withholds every status control.
    let releaseOverlap, releaseOverview;
    const heldOverlap = new Promise(resolve => { releaseOverlap = resolve; });
    const heldOverview = new Promise(resolve => { releaseOverview = resolve; });
    await page.route('**/api/set_client_active*', async route => { await heldOverlap; await route.continue(); }, { times: 1 });
    await page.route('**/api/client_overview*', async route => { await heldOverview; await route.continue(); }, { times: 1 });
    await committed.getByRole('button', { name: 'Restore', exact: true }).click();
    await search.fill('Acme Corp');
    await acmeArchive.click();
    await expect(archiveDialog('Acme Corp')).toBeVisible();
    const reloadRequested = page.waitForRequest(request => request.url().includes('/api/client_overview'));
    const org = sql(`SELECT org_id FROM clients WHERE id = '${acmeId}'`);
    releaseOverlap();
    await reloadRequested;
    // Until the reload settles the page cannot confirm authority, so an idle
    // confirmation closes (only an archive already in flight stays open).
    await expect(archiveDialog('Acme Corp')).toBeHidden();
    sql(`UPDATE organizations SET permission_policy_version = 1 WHERE id = '${org}'`);
    try {
      releaseOverview();
      await expect(page.getByRole('alert').filter({ hasText: 'Could not load clients' })).toBeVisible();
      await expect(archiveDialog('Acme Corp')).toBeHidden();
      for (const control of ['Archive', 'Restore', 'Archive client']) {
        await expect(page.getByRole('button', { name: control, exact: true })).toHaveCount(0);
      }
    } finally {
      sql(`UPDATE organizations SET permission_policy_version = 0 WHERE id = '${org}'`);
    }
    await page.reload();
    assert.equal(clientActive(committedId), 't');
    assert.equal(clientActive(acmeId), 't');

    // An edit that renames a client out of the current search refocuses the heading.
    await search.fill(committedName);
    await committed.getByRole('button', { name: 'Edit', exact: true }).click();
    await field('Client name').fill(`Renamed away ${Date.now()}`);
    await dialog.getByRole('button', { name: 'Save changes', exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(committed).toHaveCount(0);
    await expect(page.locator('#clients-heading')).toBeFocused();
    sql(`UPDATE clients SET name = '${committedName.replaceAll("'", "''")}' WHERE id = '${committedId}'`);
    await page.reload();
    console.log('PASS: archive and restore are guarded while pending with visible feedback; focus returns to the live control or heading, never overrides the user, and an open confirmation closes under changed authority');

    // Client Detail offers the same actions in its header.
    await page.goto(`${base}/clients/${committedId}`);
    const status = page.locator('#client-status');
    await expect(status).toHaveText('Archive client');
    await status.click();
    await archiveDialog(committedName).getByRole('button', { name: 'Archive client', exact: true }).click();
    await expect(archiveDialog(committedName)).toBeHidden();
    await expect(page.getByText('Archived', { exact: true })).toBeVisible();
    await expect(status).toHaveText('Restore client');
    await expect(status).toBeFocused();
    await expect(page.getByRole('status').filter({ hasText: `${committedName} archived` })).toBeVisible();
    let releaseDetail;
    const heldDetail = new Promise(resolve => { releaseDetail = resolve; });
    await page.route('**/api/set_client_active*', async route => { await heldDetail; await route.continue(); }, { times: 1 });
    await status.click();
    await expect(status).toHaveText('Restoring client…');
    await expect(status).toBeDisabled();
    releaseDetail();
    await expect(status).toHaveText('Archive client');
    await expect(page.getByRole('status').filter({ hasText: `${committedName} restored` })).toBeVisible();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(status).toBeFocused();
    assert.equal(clientActive(committedId), 't');
    assert.equal(history(), before);
    await page.goto(`${base}/clients/${acmeId}`);
    await status.click();
    await archiveDialog('Acme Corp').getByRole('button', { name: 'Archive client', exact: true }).click();
    await expect(archiveDialog('Acme Corp').getByRole('alert')).toContainText("Archive this client's projects before archiving the client.");
    await page.keyboard.press('Escape');
    await expect(status).toHaveText('Archive client');
    assert.equal(history(), before);
    console.log('PASS: Client Detail archives with confirmation, restores directly and refuses while projects are active');
    await page.goto(`${base}/clients`);
    await page.getByRole('link', { name: 'Import', exact: true }).click();
    await expect(page).toHaveURL(`${base}/admin/importers`);
    await expect(page.getByRole('heading', { name: 'Importers', exact: true })).toBeVisible();
    console.log('PASS: the existing administrator import entry point remains reachable');
    assert.deepEqual(errors, [], 'no uncaught browser errors');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exit(1); });
