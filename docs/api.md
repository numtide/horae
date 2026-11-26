# Horae API v1

`/api/v1` lets a script or another application read and log time in Horae as one person, with that
person's permissions. Everything it does goes through the same rules as the web app: a token can
never do more than its owner can in the browser.

The complete reference is the OpenAPI document each instance serves at
`https://<your-instance>/api/v1/openapi.json`. This guide covers what the document cannot say on
its own.

## First call

1. In Horae, open **My settings → API tokens**, name a token (for example "calendar sync") and
   create it. Copy it right away: Horae stores only a digest and will not show it again.

1. Call the API with it:

   ```sh
   export H=https://time.example.com T=horae_pat_…
   curl -s -H "Authorization: Bearer $T" "$H/api/v1/me"
   ```

   The answer names you, your organization and the token.

A token can have an expiry date. Revoke it from the same page when a script no longer needs it.
Administrators see and revoke everyone's tokens under **Administration → API tokens**, which also
shows when each token was last used.

## Apps

An app that people sign in to, such as a desktop or phone timer, does not ask for a personal token.
It gets an access token (`horae_oat_…`) through OAuth 2.0 with PKCE, from the endpoints that
`https://<your-instance>/.well-known/oauth-authorization-server` lists, and sends it the same way.
An access token acts as the person who approved the app, limited to the scopes they granted:

| Scope | Operations |
| --- | --- |
| `time:read` | `GET /me/timer`, `GET /time_entries`, `GET /time_entries/{id}` |
| `time:write` | every time entry and timer write; includes `time:read` |
| `catalog:read` | `GET /me/trackable` and the catalogue reads |

`GET /me` takes any scope, and its `token` names the app and the granted scopes. Its `profile` is
the person's permission profile, and `administrator` is the token's, so an administrator's app
token shows their administrator profile with `"administrator": false`. A request outside
the token's scopes gets `403` with code `insufficient_scope` and a
`WWW-Authenticate: Bearer error="insufficient_scope", scope="…"` header naming the scope that would
do. Within its scopes, a token still can do no more than its person can in the web app, and never
acts as an administrator. The OpenAPI document lists each operation's scope. No scope covers the
invoices yet: they answer an app's token with `insufficient_scope`, and need a personal token.

## Requests

| Topic | Rule |
| --- | --- |
| Authentication | `Authorization: Bearer horae_pat_…` (or an app's `horae_oat_…`) on every operation except `openapi.json`. Cookies and query-string tokens are not accepted |
| Body | `Content-Type: application/json`. A field the operation does not know is refused (`422 unknown_field`) |
| Ids | UUID strings |
| Durations | whole minutes, in fields ending in `_minutes` or named `minutes` |
| Money | `{ "amount_minor": 1250, "currency": "EUR" }`, in minor units |
| Instants | RFC 3339 in UTC, such as `2026-11-18T09:30:00Z` |
| Dates | `YYYY-MM-DD`. A time entry's date is the person's calendar day, as on the timesheet |
| Unknown query parameters | ignored |

A missing, unknown, revoked or expired token, and a token whose owner was deactivated, all get the
same `401` with code `unauthenticated` and a `WWW-Authenticate: Bearer error="invalid_token"`
header (just `Bearer` when no token was sent).

## What you can do

| Method and path | What it does |
| --- | --- |
| `GET /me` | the token's owner, organization and token |
| `GET /me/trackable` | the project and task pairs the owner may track time to today |
| `GET /me/timer` | the owner's running timer, or `204` when none runs |
| `GET /time_entries`, `GET /time_entries/{id}` | the entries the owner may see: their own, those of people they manage, or all, as on the timesheet |
| `POST /time_entries` | record time with a duration (`201`, with a `Location` header) |
| `PATCH /time_entries/{id}` | change some fields of an entry |
| `DELETE /time_entries/{id}` | delete an entry (`204`) |
| `POST /timers` | start a timer on a new entry dated today (`201`) |
| `POST /time_entries/{id}/stop` | stop a running timer |
| `POST /time_entries/{id}/restart` | run a stopped entry of today again |
| `GET /projects`, `/tasks`, `/clients`, `/people` (and `/{id}`) | the catalogue, read-only |
| `GET /invoices`, `GET /invoices/{id}` | the organization's invoices, read-only, for an owner who may view all invoices; one invoice comes with its lines. Filters: `status`, `client_id`, `updated_since` |
| `GET /invoices/{id}/pdf` | the invoice's PDF, the web app's download byte for byte |

All paths are under `/api/v1`. Time entries and timers behave as on the timesheet:

- Starting a timer, or restarting an entry, first stops a timer that is already running.
- Stopping adds the elapsed time to the minutes the entry already had. As on the timesheet, the
  daily limit may cut it.
- Only an entry dated today, in the person's time zone, can be restarted. Any other gives `409`.
- An entry that has been submitted, approved or invoiced cannot be changed or deleted (`409`, with
  the reason the timesheet shows).
- In a `PATCH`, an absent field keeps its value. `null` clears `notes` and `start_minute`; on the
  other fields it also keeps the value.
- To write for someone else, pass `person_id`. The owner needs the same permission over that
  person as in the web app. Otherwise the answer is `403`.
- An entry the owner cannot see is `404`, for reads and writes alike.

## Lists and cursors

Every list answers one page:

```json
{ "data": [ … ], "next_cursor": "…" }
```

- `limit` sets the page size, from 1 to 1000. The default is 100.
- Pass `next_cursor` back as `cursor` for the next page. It is `null` on the last page.
- Items come in `id` order, which is creation order. While nothing changes underneath, a walk
  returns each record once.
- A cursor belongs to its list and its filters, and is valid for 24 hours. Changing a filter or
  sending an old cursor gives `400 invalid_cursor`: start the walk again without a cursor.

```sh
next=""
while :; do
  page=$(curl -s -H "Authorization: Bearer $T" "$H/api/v1/time_entries?limit=1000${next:+&cursor=$next}")
  jq -c '.data[]' <<<"$page"
  next=$(jq -r '.next_cursor // empty' <<<"$page")
  [ -z "$next" ] && break
done
```

Each list's filters are in the OpenAPI document. Time entries take `person_id`, `project_id`,
`task_id`, `client_id`, `from` and `to` (the spent date, inclusive), `is_running` and
`updated_since`.

## Incremental sync

`updated_since` (an RFC 3339 instant) returns only the records changed at or after it. A sync that
runs every hour:

1. Note the time the run **starts**.
1. Walk the list with `updated_since` set to the start time of the previous run.
1. Save this run's start time for the next one.

Using the start time, not the end, means a record changed while the walk was running is fetched
again next time instead of being missed. `updated_since` does not report deletions. To notice
them, compare a full walk of the list with what you hold.

## Errors

Every error is `application/problem+json`:

```json
{
  "type": "about:blank",
  "title": "Conflict",
  "status": 409,
  "detail": "This person already has a running timer",
  "code": "conflict",
  "field": null
}
```

`detail` is the reason the web app shows for the same refusal. `field` names the input at fault,
when there is one. Program against `status` and `code`, not `detail`.

| Status | `code` | When |
| --- | --- | --- |
| 400 | `bad_request` | malformed JSON, a body that is not JSON or lacks a required field, a parameter of the wrong type |
| 400 | `invalid_cursor` | a cursor from another list or other filters, altered, or older than 24 hours |
| 401 | `unauthenticated` | no token, or a token that is not valid (see above) |
| 403 | `forbidden` | the owner may not do this in the web app |
| 403 | `insufficient_scope` | an app's token lacks the operation's scope (see [Apps](#apps)) |
| 404 | `not_found` | missing, or outside what the owner may see |
| 409 | `conflict` | the record's state forbids it: submitted, approved, invoiced, no running timer, … |
| 413 | `too_large` | an invoice too large for its PDF |
| 422 | `invalid`, `unknown_field` | a value fails validation, or the body has a field the operation does not take |
| 429 | `rate_limited` | over the token's limit; wait the seconds in `Retry-After` |
| 500 | `internal` | a server error; `detail` is withheld |
| 503, 504 | `unavailable`, `timeout` | too many PDFs being made at once, or one that took too long; try again |

A later release may add codes. Treat an unknown `code` as its status.

## Limits

Each token may make 100 requests per 15 seconds. An app's access tokens share one limit per
approval, so refreshing does not reset it. The Harvest-compatible API counts against the same
limit. Over it, the answer is `429` with a `Retry-After` header in seconds. Responses carry no other
rate headers. An administrator can change the limit with `HORAE_API_RATE_LIMIT`
(`services.horae.api.rateLimit` on NixOS).

## The OpenAPI document

`GET /api/v1/openapi.json` needs no token. It describes every operation, parameter, schema and
error, so you can generate a client from it:

```sh
curl -s https://time.example.com/api/v1/openapi.json > horae.json
openapi-generator-cli generate -i horae.json -g python -o horae-client --package-name horae_client
```

The repository's `api-v1` check builds exactly such a client and runs it against a test instance.

## Compatibility

Within v1, changes are only additions: new operations, new optional parameters, new response
fields and new error codes. A client must ignore response fields it does not know. Anything that
would break a v1 client goes into a v2.
