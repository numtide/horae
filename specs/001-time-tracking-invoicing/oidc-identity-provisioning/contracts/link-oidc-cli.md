# Contract: `horae user link-oidc` and `horae user list`

Local operator commands. They connect to PostgreSQL with `DATABASE_URL` like the
other `user` commands, need no running server and no session file, and are not
exposed over HTTP, the UI or plugins (constitution 3.0.0, Principle IV exception).

## `horae user link-oidc`

```text
horae user link-oidc --user-id <UUID> --issuer <URL> --subject <ID>
```

All three flags are required. `--user-id` must parse as a UUID (Clap rejects
anything else with exit 2).

The command applies the transition table in [data-model.md](../data-model.md) in
one transaction that holds the target row lock. It does not run migrations.

### Outcomes

| Outcome | Stream | Text | Exit |
|---------|--------|------|------|
| Linked | stdout | `Linked user <UUID> to the OIDC identity.` | 0 |
| Unchanged | stdout | `User <UUID> is already linked to this OIDC identity; nothing changed.` | 0 |
| IssuerRecorded | stdout | `Recorded the issuer on the existing OIDC binding of user <UUID>.` | 0 |
| UserNotFound | stderr | `No user with ID <UUID>.` | 1 |
| Inactive | stderr | `User <UUID> is deactivated; reactivate the account before linking it.` | 1 |
| SubjectInUse | stderr | `Another user is already linked to this subject.` | 1 |
| AlreadyLinked | stderr | `User <UUID> is already linked to a different OIDC identity. Horae does not replace bindings.` | 1 |
| InvalidSubject | stderr | `The subject must be 1–255 bytes with no leading or trailing whitespace.` | 1 |
| InvalidIssuer | stderr | `The issuer must be an https URL (or http on a loopback host) with no query or fragment.` | 1 |
| UnreadableConfiguredIssuer | stderr | `HORAE_OIDC_ISSUER is not valid UTF-8; the issuer cannot be checked.` (the value is never printed) | 1 |
| IssuerMismatch | stderr | `The issuer does not match HORAE_OIDC_ISSUER exactly.` (only when `HORAE_OIDC_ISSUER` is set and non-empty) | 1 |
| Database or other error | stderr | `Error: …` (existing `anyhow` reporting) | 1 |
| Usage error | stderr | Clap usage text | 2 |

No outcome prints or logs the issuer or the subject. The account's role, email,
name, rates and active status are never written. No session is created or ended.

## `horae user list`

Adds the account ID as the first column; nothing else changes.

```text
ID                                    NAME                           EMAIL                          ROLE       STATUS
0190…                                 Ops                            ops@contoso.example            admin      active
```

No issuer or subject is printed.

## Sign-in resolution (no interface change)

`/auth/login`, `/auth/oidc/start` and `/auth/callback` keep their routes, redirects
and responses. Only which account a validated token resolves to changes:

1. Account whose recorded issuer equals the token's `iss` and whose subject equals
   its `sub`. Active → session; deactivated → denied.
1. Otherwise, only when `email_verified` is present and true: the active account
   with that email and no binding at all. It is bound to (`iss`, `sub`) by a
   conditional update and receives a session. A competing callback that loses is
   denied unless it carries the same `iss` and `sub` for the same active account.
1. Otherwise denied, with no session and no account change. This includes a legacy
   binding (subject with no issuer) and a binding with another issuer.

If step 2's update collides with the unique subject because another account already
holds `sub` (a legacy or other-issuer binding, or a concurrent `link-oidc`), the
callback keeps its existing behavior: the resolution error is logged by the existing
line (its displayed text must not contain the subject, which T014 asserts), the browser is redirected to `/auth/login`, no session is created and no
binding changes. No new error mapping is added on this path. Only `link-oidc` maps
that unique violation (SQLSTATE 23505 on the subject constraint) to `SubjectInUse`.
