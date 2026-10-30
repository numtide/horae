# Data Model: Operator-provisioned OIDC identity binding

## `users` (existing table, one new column)

| Column | Change | Rule |
|--------|--------|------|
| `oidc_subject text UNIQUE NULL` | unchanged | Stored byte for byte. Unique across all accounts. |
| `oidc_issuer text NULL` | **new** (migration `0049_oidc_issuer.sql`) | Stored exactly as validated; no normalization. |
| — | **new** constraint `users_oidc_issuer_requires_subject` | `CHECK (oidc_issuer IS NULL OR oidc_subject IS NOT NULL)` |

No backfill. No index beyond the existing unique index on `oidc_subject`, which
serves both lookups. `org_id` is already on the table (Principle III).

`oidc_issuer` is not added to `models::User`, `UserListItem` or any page query. It is
read only by `auth/oidc.rs` and `auth/oidc/link.rs`. It is not granted to the plugin
role, and plugin validation rejects a role that can read it.

## Binding states of one account

| State | `oidc_subject` | `oidc_issuer` | Sign-in with validated (I, S) |
|-------|----------------|---------------|-------------------------------|
| Unbound | NULL | NULL | Only via verified email (binds to (I, S)) |
| Legacy | S₀ | NULL | Never matches; denied with no write until linked |
| Bound | S₀ | I₀ | Matches only when S = S₀ and I = I₀ |

`active = false` denies sign-in in every state, and every request re-checks it.

## Transitions

Sign-in (callback), unchanged except for the issuer:

| From | Trigger | To |
|------|---------|----|
| Unbound, active | validated token, `email_verified` true, email matches | Bound (I, S) |
| Legacy | any sign-in | Legacy, sign-in denied (never adopts an issuer) |
| Bound (I₀, S₀) | token (J, S₀), J ≠ I₀ | Bound (I₀, S₀), sign-in denied |
| Unbound M, while another account L holds S | token (I, S), verified email of M | Unchanged; the conditional UPDATE hits the unique subject index and the callback keeps its existing error path: logged, redirected to sign-in, no session, M stays unbound, L unchanged |

`horae user link-oidc --user-id U --issuer I --subject S`, with U's row locked:

| # | Target U | Other account holds S | Result | Exit |
|---|----------|-----------------------|--------|------|
| 1 | does not exist | — | reject `UserNotFound` | 1 |
| 2 | `active = false` | — | reject `Inactive` | 1 |
| 3 | Bound (I, S) | — | `Unchanged` | 0 |
| 4 | Legacy (S) | — | `IssuerRecorded` → Bound (I, S) | 0 |
| 5 | Unbound | no | `Linked` → Bound (I, S) | 0 |
| 6 | Unbound | yes (holder id ≠ U) | reject `SubjectInUse` | 1 |
| 7 | Bound (I′, S), I′ ≠ I | — | reject `AlreadyLinked` | 1 |
| 8 | Bound or Legacy with S′ ≠ S | — | reject `AlreadyLinked` | 1 |

Checks run in this order: input validation (before any database access), then
rows 1–2, then 3–8. "Other account holds S" means a holder whose id differs from
U; when U itself holds S (rows 3–4) the result is `Unchanged` or `IssuerRecorded`,
never `SubjectInUse`. A bound or legacy U whose subject differs from S is row 8
whether or not S is held elsewhere. Rejections write nothing. Only the target row is locked; between different
targets the unique index on `oidc_subject` decides, and only its unique violation
(SQLSTATE 23505 on that constraint) maps to row 6.

Input validation (rejects, exit 1, before database access):

- subject: non-empty; at most 255 bytes in UTF-8 (so a 128-character subject of
  2-byte characters is rejected); rejected when `subject.trim() != subject`, i.e.
  leading or trailing Unicode `White_Space`. The value is compared, never trimmed.
- issuer: parses as an absolute URL; scheme `https`, or `http` with a loopback host
  (`localhost`, `127.0.0.0/8`, `::1`); no query, no fragment; no ASCII control
  character or backslash anywhere; exactly `//` between the scheme's `:` and the
  host (so `https:host`, `https:/host` and `https:///host` are refused, which a
  URL parser would otherwise repair). The parse is only a check: the stored value
  is the original string (case and percent-encoding included), never the parser's
  normalized form, and nothing is trimmed.
- `HORAE_OIDC_ISSUER` set but not UTF-8: refused (`UnreadableConfiguredIssuer`),
  never skipped; empty or unset means no configured issuer.
- issuer equals `HORAE_OIDC_ISSUER` exactly when that variable is set and non-empty
  in the command's environment, whatever the other OIDC settings (research D4).

"Before database access" is literal: `main.rs` opens the pool before it matches the
`user` action today, so `link-oidc` runs these checks before `db::create_pool`.
