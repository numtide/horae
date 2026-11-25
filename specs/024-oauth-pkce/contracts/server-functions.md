# Contract: server functions and pages (PRs C, D, F)

Mutations from the web app go through these server functions, which call `ops::oauth`
(constitution IV). They live in `crates/horae/src/server_fns/oauth.rs`, and the administration's
in `oauth_apps.rs` beside it. Errors map `OpError` to the
named status constants, as elsewhere.

## Approval (PR C)

| Server function | Who | Input | Output | Errors |
| --- | --- | --- | --- | --- |
| `oauth_consent_details(request)` | signed-in person | request id | `ConsentDetails` | `NOT_FOUND` when the request is unknown or expired, or the client was disabled since |
| `approve_oauth_request(request)` | signed-in person | request id | redirect URL with `code`, `state`, `iss` | `NOT_FOUND` as above; `FORBIDDEN` for an inactive person |
| `deny_oauth_request(request)` | signed-in person | request id | redirect URL with `error=access_denied` | `NOT_FOUND` as above |

All three read the pending request from the session; approve and deny remove it, so a second click
finds nothing.

### Page `OAuthConsent` (`/oauth/consent?request=<id>`)

Outside the app shell (no sidebar, no timer), centered card in DESIGN.md tokens, like the sign-in
page in spirit but built from the app's components:

- Horae mark, then the title: "<App name> wants to use your Horae account".
- A line under it: "Registered by your workspace" or "Horae's own app" (built-in).
- The account: person name and organization, with "Not you? Sign out" (posts to `/auth/logout`,
  then back to the same request through `return_to`).
- "It will be able to:" and one line per scope (catalog sentences, FR-060), with an icon each.
- "After you allow, Horae sends you back to <host or scheme>."
- Footer: Primary "Allow", secondary "Deny". Allow is busy while the call runs; both disabled after
  the first click.
- States: loading skeleton; "This request has expired. Go back to the app and start again."; load
  error with retry.
- No toast; the page leaves on success.

## Connected apps in My settings (PR F)

| Server function | Who | Output | Errors |
| --- | --- | --- | --- |
| `list_my_oauth_grants()` | any signed-in person | own active grants (`GrantRow`), newest first | |
| `revoke_my_oauth_grant(id)` | the grant's person | the row | `NOT_FOUND` for anyone else's |

Page `SettingsApps` at `/settings/apps`, in the SETTINGS group after "API tokens", titled "Connected
apps": an intro sentence, a table (app, access, allowed on, last used) and a "Revoke…" action per
row through `ConfirmDialog` (danger tone). Empty state: "No apps connected yet". No link to the
audit log (C-4).

## Apps in administration (PR D)

| Server function | Who | Input | Output | Errors |
| --- | --- | --- | --- | --- |
| `list_oauth_apps()` | administrator | | `Vec<AppRow>` | `FORBIDDEN` |
| `get_oauth_app(id)` | administrator | | `AppRow` | `FORBIDDEN`; `NOT_FOUND` |
| `oauth_apps_can_connect()` | signed-in person | | whether `HORAE_PUBLIC_URL` is set | |
| `create_oauth_app(new)` | administrator | name, kind, redirect URIs, scopes | `CreatedApp` (secret once for confidential) | `BAD_REQUEST` with the field on invalid input |
| `update_oauth_app(id, changes)` | administrator | name, redirect URIs, scopes | `AppRow` | `BAD_REQUEST`; `CONFLICT` for the built-in client and for a name another app has (field `name`) |
| `set_oauth_app_enabled(id, enabled)` | administrator | | `AppRow` | |
| `replace_oauth_app_secret(id)` | administrator | | the new secret once | `CONFLICT` for a public client |
| `delete_oauth_app(id)` | administrator | | | `CONFLICT` for the built-in client |
| `list_oauth_app_grants(id)` | administrator | | `Vec<GrantRow>` | |
| `revoke_oauth_grant(id)` | administrator | | nothing; the page reloads the list | |

Narrowing an app's scopes does not change existing grants; their tokens keep the scopes approved
until revoked. Removing a redirect URI does not affect issued tokens.

Pages (admin shell, after "API tokens"):

- `AdminApps` at `/admin/apps`: a table of apps (name with its type under it, scopes, status, active authorizations as People),
  a Primary "Register app" button, the built-in Android client first with an Enable/Disable action.
  When `HORAE_PUBLIC_URL` is unset, a notice explains that apps cannot connect until the operator
  sets it.
- `AdminApp` at `/admin/apps/:id`: details, client id with Copy, redirect URIs, scopes; Actions menu
  (Edit, Replace secret…, Disable/Enable, Delete… last as a danger item); below, the app's
  authorizations with person, access, allowed on, last used and "Revoke…".
- Register and edit in a `Modal` form: name, type (radio: "Installed on people's devices" /
  "Runs on a server"), redirect URIs (one per line, validated on the server), scopes (checkboxes).
  The secret of a confidential app appears once after creation, in the one-time reveal component
  used by API tokens.

## Catalog

New sections in `crates/i18n/locales/en/horae.ftl`: `## OAuth approval` (ids `oauth-*`) and
`## Apps` (ids `apps-*`), following the file's header. Scope sentences are `oauth-scope-time-read`,
`oauth-scope-time-write`, `oauth-scope-catalog-read`, shared by the approval page, My settings and
administration. The authorize error page served by `horae-api` takes its text from the same
catalog through `horae-i18n` (English until a language is negotiated there).
