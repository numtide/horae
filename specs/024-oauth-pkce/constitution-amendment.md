# Constitution amendment for 024 OAuth 2.0 with PKCE

**Status**: Approved by the maintainer on 2026-11-22 (spec D-3), as drafted. Not applied yet: PR A
applies this text to `.specify/memory/constitution.md` with a Sync Impact Report.

**Version change**: 7.0.0 → 7.1.0 (MINOR: materially expanded guidance inside Principle IV; no
principle is removed or redefined). The maintainer chose MINOR.

## Why

Since 7.0.0, Principle IV's *Token-authenticated APIs* clause lets the APIs act with "a personal
access token or an OAuth2 access token that Horae issued", and says:

> Token issue and revocation are themselves mutations and go through server functions.

For personal access tokens this holds: a signed-in person creates and revokes them on a page. For
OAuth it holds for approval (the approval page calls a server function) and for revocation from My
settings and administration. It cannot hold for three steps that RFC 6749 and RFC 7009 define as
plain HTTP endpoints called by programs with no Horae session:

- exchanging an authorization code for the first tokens,
- exchanging a refresh token for new tokens (rotation),
- an app revoking its own token when it signs out.

Their authority is not a session but a credential the program presents: a code with its PKCE
verifier, a refresh token, or a client secret. The spec excludes the alternative that would avoid
this (tokens handed out by the approval page through the redirect, the implicit grant), because
RFC 9700 forbids it.

## Proposed text

### Principle IV: in the "Token-authenticated APIs" clause, replace the last bullet

Current:

> - Token issue and revocation are themselves mutations and go through server functions.

Proposed:

> - Token issue and revocation are themselves mutations and go through server functions, with one
>   bounded exception for OAuth2. Horae's OAuth2 token and revocation endpoints MAY invoke the
>   shared OAuth operations, without a session, to exchange an authorization code for tokens,
>   exchange a refresh token for new tokens, and revoke a token, with the presented credential as
>   their authority: a single-use code together with its PKCE verifier, a single-use refresh
>   token, or a client's secret. They MUST NOT create or widen an authorization (only a person's
>   approval through a server function does), MUST NOT issue a token with more scopes than that
>   approval, MUST NOT write anything but OAuth credential and authorization state and its audit
>   records, and MUST store every credential only as a hash.

### Rationale: append one sentence

> OAuth2 clients redeem codes and refresh tokens without a session by design; bounding what those
> endpoints may write keeps the authorization a person gave as the only source of a token's
> authority.

## Effect on other text

- Technology & Data Constraints: no change. Scopes narrow stored permission state and are not a
  second authorization model; the spec's FR-063 and the plan's Constitution Check record that.
- Templates: no change expected; to be checked in the Sync Impact Report.
- AGENTS.md: the *Auth* paragraph gains a sentence on OAuth with the feature's PR C, not with this
  amendment.
