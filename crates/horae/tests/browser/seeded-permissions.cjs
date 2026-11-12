// `horae seed` gives the demo admin an Administrator permission state. A suite
// that builds its own permission states sets the seeded ones aside and puts
// them back once its own rows are gone, so every suite begins from the demo
// exactly as the seed left it.
const { execFileSync } = require('node:child_process');

const sql = query => execFileSync('psql', [process.env.DATABASE_URL, '-X', '-v', 'ON_ERROR_STOP=1', '-qAt', '-c', query], { encoding: 'utf8' }).trim();

// Removes the organization's permission states and returns the function that restores them.
const setAsideSeededStates = org => {
  const saved = sql(`WITH removed AS (DELETE FROM person_permission_states WHERE org_id='${org}' RETURNING *)
    SELECT coalesce(json_agg(removed), '[]') FROM removed`);
  return () => sql(`INSERT INTO person_permission_states
    SELECT * FROM json_populate_recordset(NULL::person_permission_states, $seeded$${saved}$seeded$::json)`);
};

// The development login picks an active Administrator. A suite that takes the
// demo admin's Administrator state away signs `context` in first.
const signInAsAdmin = async (context, base) => {
  const login = await context.request.post(`${base}/auth/dev-login`, { maxRedirects: 0 });
  if (login.status() !== 303) throw new Error(`Development login answered ${login.status()}`);
};

module.exports = { setAsideSeededStates, signInAsAdmin };
