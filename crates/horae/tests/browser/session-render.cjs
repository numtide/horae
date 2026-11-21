// The server renders the signed-in shell: the first HTML of a page already has
// the person's rail, its permission-dependent rows and the admin workspace,
// before any script runs. Uses only the design runner's disposable database.
const { request } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { setAsideSeededStates, signInAsAdmin, devLoginUser } = require('./seeded-permissions.cjs');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base), database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.match(database.searchParams.get('host') || '', /^\/tmp\/horae-browser\.[A-Za-z0-9]+$/);
const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();
const state = '019f5300-0000-7000-8000-000000000001';
const floor = ['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own'];

// The visible text of the server's <aside> (the rail) and <main>, in order.
const regions = html => Object.fromEntries(['aside', 'main'].map(tag => {
  const inner = html.match(new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)</${tag}>`))?.[1] ?? '';
  const text = inner.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<!--[\s\S]*?-->/g, '')
    .split(/<[^>]+>/).map(part => part.trim()).filter(Boolean);
  return [tag, text];
}));

(async () => {
  const signedIn = await request.newContext({ baseURL: base });
  const signedOut = await request.newContext({ baseURL: base });
  const page = async (context, path) => {
    const response = await context.get(path, { headers: { accept: 'text/html' }, maxRedirects: 0 });
    return { status: response.status(), location: response.headers().location, html: await response.text() };
  };
  let restoreSeededStates;
  try {
    // A signed-out page load still goes to the sign-in page.
    const out = await page(signedOut, '/timesheet');
    assert.equal(out.status, 303);
    assert.match(out.location, /\/auth\/login$/);

    await signInAsAdmin(signedIn, base);
    const actor = devLoginUser();
    const name = sql(`SELECT name FROM users WHERE id='${actor.id}'`);
    const org = sql(`SELECT name FROM organizations WHERE id='${actor.org_id}'`);

    // A dated week: `/timesheet` alone redirects in the browser and renders nothing on the server.
    const week = `/timesheet/week/${process.env.HORAE_TEST_WEEK}`;
    const sheet = regions((await page(signedIn, week)).html);
    for (const text of [name, 'People', 'Clients', 'Tasks', 'Invoices', 'Approvals']) {
      assert.ok(sheet.aside.includes(text), `the rail lacks ${text}: ${sheet.aside.join(' | ')}`);
    }
    assert.ok(!sheet.aside.includes('Not signed in'), sheet.aside.join(' | '));

    const admin = regions((await page(signedIn, '/admin/importers')).html);
    assert.ok(admin.main.includes(org), `the workspace lacks its name: ${admin.main.join(' | ')}`);
    for (const text of ['Importers', 'Audit log', 'API tokens']) {
      assert.ok(admin.main.includes(text), `the admin navigation lacks ${text}: ${admin.main.join(' | ')}`);
    }
    assert.ok(!admin.main.some(text => text.startsWith('Loading')), admin.main.join(' | '));
    console.log('PASS: the first HTML has the signed-in rail and the admin workspace; signed out redirects');

    // The same person as a member: the rows that need grants are not in the HTML.
    restoreSeededStates = setAsideSeededStates(actor.org_id);
    sql(`INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
      VALUES ('${state}','${actor.org_id}','${actor.id}',1,ARRAY[${floor.map(grant => `'${grant}'`).join(',')}],false,'individual')`);
    const member = regions((await page(signedIn, week)).html);
    assert.ok(member.aside.includes(name), member.aside.join(' | '));
    for (const text of ['People', 'Clients', 'Tasks', 'Invoices', 'Approvals']) {
      assert.ok(!member.aside.includes(text), `a member's rail shows ${text}: ${member.aside.join(' | ')}`);
    }
    assert.ok(member.aside.includes('Projects') && member.aside.includes('Reports'), member.aside.join(' | '));
    console.log('PASS: a member\'s first HTML leaves out the rows their grants do not open');
  } finally {
    if (restoreSeededStates) {
      sql(`DELETE FROM person_permission_states WHERE id='${state}'`);
      restoreSeededStates();
    }
    await signedIn.dispose();
    await signedOut.dispose();
  }
})().catch(error => {
  console.error(error);
  process.exit(1);
});
