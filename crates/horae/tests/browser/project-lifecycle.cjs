// All fixtures and role changes belong to the runner's disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.ok(database.pathname === '/horae' && database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], { encoding: 'utf8' }).trim();
const project = '01970000-0000-7000-8000-000000000501';
const entry = '01970000-0000-7000-8000-000000000502';
const invoice = '01970000-0000-7000-8000-000000000503';
const admin = sql("SELECT id FROM users WHERE email='admin@example.com'");
const history = () => JSON.parse(sql(`SELECT json_build_object(
  'project', to_jsonb(p) - 'active',
  'entries', (SELECT jsonb_agg(to_jsonb(e) ORDER BY e.id) FROM time_entries e WHERE e.project_id=p.id),
  'assignments', (SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM assignments a WHERE a.project_id=p.id),
  'tasks', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.task_id) FROM project_tasks t WHERE t.project_id=p.id),
  'invoice', (SELECT to_jsonb(i) FROM invoices i WHERE i.id='${invoice}'),
  'lines', (SELECT jsonb_agg(to_jsonb(l) ORDER BY l.id) FROM invoice_line_items l WHERE l.invoice_id='${invoice}')
) FROM projects p WHERE p.id='${project}'`));

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  const mutations = [];
  let release;
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    if (request.url().includes('/api/set_project_active')) mutations.push(request);
  });
  const active = () => sql(`SELECT active FROM projects WHERE id='${project}'`);
  const details = page.getByRole('region', { name: 'Project details', exact: true });
  const actions = details.locator('#project-detail-actions-trigger');
  async function openStatus(verb) {
    await actions.scrollIntoViewIfNeeded();
    await actions.focus();
    // Finish viewport/focus scrolling before testing a menu that dismisses on scroll.
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await page.keyboard.press('Enter');
    await page.getByRole('menuitem', { name: verb, exact: true }).focus();
    await page.keyboard.press('Enter');
    const dialog = page.getByRole('dialog', { name: `${verb} project?`, exact: true });
    await expect(dialog).toBeVisible();
    await expect.poll(() => dialog.evaluate(node => node.contains(document.activeElement))).toBe(true);
    return dialog;
  }
  try {
    sql(`BEGIN;
      INSERT INTO projects (id,org_id,client_id,name,code,currency,budget_kind,budget_minutes)
        SELECT '${project}',c.org_id,c.id,'Lifecycle fixture','LIFECYCLE',c.currency,'hours',240
        FROM clients c JOIN organizations o ON o.id=c.org_id WHERE o.name='Demo Org' ORDER BY c.id LIMIT 1;
      INSERT INTO project_tasks (project_id,task_id,billable)
        SELECT p.id,t.id,true FROM projects p JOIN tasks t ON t.org_id=p.org_id AND t.name='Development' WHERE p.id='${project}';
      INSERT INTO assignments (id,project_id,user_id)
        VALUES ('01970000-0000-7000-8000-000000000504','${project}','${admin}');
      INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
        SELECT '${invoice}',org_id,client_id,'LIFECYCLE-INV',CURRENT_DATE,CURRENT_DATE+30,currency,10000 FROM projects WHERE id='${project}';
      INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,rounded_minutes,billable,state,invoice_id,notes)
        SELECT '${entry}',p.org_id,'${admin}',p.id,t.id,CURRENT_DATE,60,60,true,'invoiced','${invoice}','Retained history'
        FROM projects p JOIN tasks t ON t.org_id=p.org_id AND t.name='Development' WHERE p.id='${project}';
      INSERT INTO invoice_line_items (id,invoice_id,time_entry_id,description,minutes,rate_cents,amount_cents)
        VALUES ('01970000-0000-7000-8000-000000000505','${invoice}','${entry}','Retained line',60,10000,10000);
      COMMIT;`);
    const before = history();
    assert.equal(before.entries.length, 1);
    assert.equal(before.lines.length, 1);
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    await page.goto(`${base}/projects/${project}`);
    await expect(details.getByRole('heading', { level: 1 })).toHaveText('[LIFECYCLE] Lifecycle fixture');
    await expect(details.getByRole('link', { name: 'Edit project', exact: true })).toHaveAttribute('href', `/projects/${project}/edit`);
    await expect(page.locator('.nav-item.active').filter({ hasText: 'Projects' })).toBeVisible();

    for (const [width, size] of [[320, 16], [390, 16], [768, 16], [1440, 16], [320, 32], [1440, 32]]) {
      await page.setViewportSize({ width, height: 600 });
      await page.evaluate(font => { document.documentElement.style.fontSize = `${font}px`; }, size);
      const dialog = await openStatus('Archive');
      const bounds = await dialog.boundingBox();
      assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width + 1 && bounds.y >= 0 && bounds.y + bounds.height <= 601);
      assert.ok(await dialog.evaluate(node => node.scrollWidth <= node.clientWidth + 1), 'Confirmation must not clip enlarged text');
      await page.keyboard.press('Escape');
      await expect(dialog).toBeHidden();
      await expect(actions).toBeFocused();
      assert.equal(active(), 't');
      console.log(`PASS: project status confirmation keyboard/cancel at ${width}px with ${size}px text`);
    }
    assert.equal(mutations.length, 0, 'Opening/cancelling must not mutate status');
    await page.evaluate(() => { document.documentElement.style.fontSize = ''; });
    await page.setViewportSize({ width: 1440, height: 900 });
    const cancelled = await openStatus('Archive');
    await cancelled.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(cancelled).toBeHidden();
    await expect(actions).toBeFocused();
    assert.equal(mutations.length, 0);
    await page.route('**/api/set_project_active*', route => route.abort(), { times: 1 });
    const dialog = await openStatus('Archive');
    const confirm = dialog.getByRole('button', { name: 'Archive project', exact: true });
    await confirm.click();
    await expect(dialog.getByRole('alert')).toContainText('Could not confirm project status');
    assert.equal(active(), 't');
    assert.deepEqual(history(), before);
    const gate = new Promise(resolve => { release = resolve; });
    await page.route('**/api/set_project_active*', async route => { await gate; await route.continue(); }, { times: 1 });
    await confirm.focus();
    await page.keyboard.press('Enter');
    await expect(dialog.getByRole('button', { name: 'Updating project…', exact: true })).toBeDisabled();
    await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    await expect(actions).toBeDisabled();
    await page.keyboard.press('Enter');
    await page.keyboard.press('Escape');
    await expect(dialog).toBeVisible();
    assert.equal(mutations.length, 2, 'Pending keyboard input must not submit again');
    release();
    await expect(dialog).toBeHidden();
    await expect(actions).toBeFocused();
    await expect(details).toContainText('Archived');
    assert.equal(active(), 'f');
    // A real status transition invalidates any previously opened editor.
    before.project.edit_revision++;
    assert.deepEqual(history(), before);
    await page.reload({ waitUntil: 'domcontentloaded' });
    await expect(details).toContainText('Archived');

    sql(`UPDATE users SET org_role='manager' WHERE id='${admin}'`);
    await page.reload({ waitUntil: 'domcontentloaded' });
    const reactivate = await openStatus('Reactivate');
    await reactivate.getByRole('button', { name: 'Reactivate project', exact: true }).click();
    await expect(reactivate).toBeHidden();
    await expect(actions).toBeFocused();
    await expect(details).toContainText('Active');
    assert.equal(active(), 't');
    before.project.edit_revision++;
    assert.deepEqual(history(), before);
    assert.equal(mutations.length, 3);

    const tasks = page.getByRole('tab', { name: /^Tasks/ });
    await tasks.focus();
    for (const [key, name] of [['ArrowRight', 'Team'], ['End', 'Invoices'], ['Home', 'Tasks']]) {
      await page.keyboard.press(key);
      const tab = page.getByRole('tab', { name: new RegExp(`^${name}`) });
      await expect(tab).toBeFocused();
      await expect(tab).toHaveAttribute('aria-selected', 'true');
    }
    sql(`UPDATE users SET org_role='member' WHERE id='${admin}'`);
    await page.reload({ waitUntil: 'domcontentloaded' });
    await expect(details.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(actions).toHaveCount(0);
    await expect(details.getByRole('link', { name: 'Edit project', exact: true })).toHaveCount(0);
    await expect(page.getByRole('tab', { name: /^Invoices/ })).toHaveCount(0);
    const denied = await context.request.post(mutations[0].url(), { data: mutations[0].postDataJSON() });
    assert.equal(denied.status(), 403);
    assert.equal(active(), 't');
    assert.deepEqual(history(), before);
    assert.deepEqual(errors, []);
    console.log('Project lifecycle: native keyboard cancel/error/pending/retry, archive/reload/reactivate, retained invoice/time/configuration, tab keys, responsive confirmation and revoked authority passed');
  } finally {
    if (release) release();
    sql(`UPDATE users SET org_role='admin' WHERE id='${admin}'`);
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
