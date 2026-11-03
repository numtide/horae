# Feature Specification: Harvest OAuth Client Settings

**Feature Branch**: `feat/harvest-oauth-client-settings`

**Created**: 2026-11-03

**Status**: Draft

**Input**: User description: "Add the ability to save the Harvest OAuth client from the frontend itself, so it does not have to be configured as environment variables (which remain supported)."

## Clarifications

### Session 2026-11-03

Decisions made before specification by the requester, recorded here as answered:

- Q: Do environment variables keep working? → A: Yes, unchanged. When the deployment sets both the client ID and the client secret in the environment, the environment client is used and the page shows it read-only as configured by the environment. Otherwise the stored client is used. A change made on the page takes effect without restarting the server.
- Q: Who may view and change the stored client? → A: Administrators only, with the same administrator check the importer already uses. Everyone else is refused as forbidden.
- Q: Where is it managed? → A: On the existing Harvest importer page, as a small form: client ID, client secret and the redirect URL to register at Harvest.
- Q: How is the secret protected? → A: It is sealed at rest with the scheme that already protects the stored Harvest tokens, keyed by the deployment's encryption key. The key itself stays in the environment only and is never stored next to what it protects. Without the key, the page explains that a secret cannot be stored and how to set the key.
- Q: Can the secret be read back? → A: No. It is write-only: the page shows only whether one is set, and offers replace and remove. The client ID may be shown.

Decisions this specification makes within that scope:

- Q: Is the redirect URL editable? → A: No, it is shown read-only with a copy action, as in the importer design. When the deployment sets it in the environment, that value is shown. Otherwise it is the address of the Horae instance the administrator is using, followed by the Harvest callback path, and it is saved with the client so the server uses the same value.
- Q: What happens to an existing connection when the client changes or is removed? → A: Harvest tokens belong to the client that issued them. Changing the client ID or removing the client therefore disconnects Harvest exactly like the existing Disconnect action: tokens are removed, and the account binding, history and imported data are retained, so only the original account can be reconnected. Replacing only the secret keeps the connection.
- Q: What about imports and authorizations in progress? → A: Any change to the stored client is refused while a Harvest import is queued or running, with the same "finish or cancel the active import" guidance Change account uses. Any change also invalidates an authorization that was started before it, so a callback arriving afterwards cannot complete with a different client.
- Q: Is the stored client per organization or per deployment? → A: Per organization, like the Harvest connection it serves. The environment client, when present, applies to every organization.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configure Harvest from the importer (Priority: P1)

An administrator of a deployment that has the encryption key but no Harvest client in the environment copies the redirect URL into Harvest, enters the client ID and secret on the importer page, saves, and can connect Harvest straight away, without a restart.

**Why this priority**: This is the feature: setting up the Harvest API import without editing the deployment.

**Independent Test**: Start the server with only the encryption key set. Save a client on the importer page, then start the connection: the authorization address carries the saved client ID and redirect URL.

**Acceptance Scenarios**:

1. **Given** no client in the environment and none stored, **When** an administrator opens the Harvest importer, **Then** the page shows the client as not configured and offers a form for the client ID and secret, and shows the redirect URL to register at Harvest, built from the current Horae address.
1. **Given** that form is filled in, **When** the administrator saves, **Then** the page shows the client ID, that a secret is set, and the redirect URL, and Connect Harvest becomes available without a restart.
1. **Given** a saved client, **When** the administrator connects Harvest, **Then** authorization and token exchange use the saved client ID, secret and redirect URL.
1. **Given** a saved client, **When** the importer is reloaded or another administrator opens it, **Then** the secret is never shown or sent to the browser; only "set" is shown.

______________________________________________________________________

### User Story 2 - Environment configuration keeps working and wins (Priority: P1)

An operator who already configures the client through the environment sees no change in behaviour, and administrators can see where the client comes from.

**Why this priority**: Existing deployments must keep working unchanged.

**Independent Test**: Start the server with the client in the environment and a different client stored for the organization. Connecting uses the environment client, and the page shows it as configured by the environment and offers no editing.

**Acceptance Scenarios**:

1. **Given** the client ID and secret are set in the environment, **When** an administrator opens the importer, **Then** the client is shown as configured by the environment, with its client ID and redirect URL, read-only.
1. **Given** the environment client and a stored client both exist, **When** Harvest is connected or an import runs, **Then** the environment client is used, and the page notes that the stored client is ignored while the environment one is set.
1. **Given** only one of the client ID and secret is set in the environment, **When** the server starts, **Then** the partial environment client is ignored as it is today, and the stored client, if any, is used.

______________________________________________________________________

### User Story 3 - Replace or remove the stored client safely (Priority: P2)

An administrator rotates the secret, switches to a different OAuth client, or removes the client, and the effect on the current Harvest connection is explained before it happens.

**Why this priority**: Credentials rotate. Doing it from the page must not leave a connection that silently fails or let an old authorization finish with the wrong client.

**Independent Test**: With Harvest connected, replace the secret only: the connection stays. Change the client ID: Harvest is disconnected and only the original account can be reconnected. Remove the client: Harvest is disconnected and the API import is unavailable, while CSV import still works.

**Acceptance Scenarios**:

1. **Given** a connected stored client, **When** the administrator replaces only the secret, **Then** the connection, account binding and history are unchanged.
1. **Given** a connected stored client, **When** the administrator saves a different client ID, **Then** the page warns first that Harvest will be disconnected; after saving, tokens are removed and the account binding, history and imported data remain.
1. **Given** a stored client, **When** the administrator removes it, **Then** the page warns first; after removal the API import shows as not configured, Harvest is disconnected as above, and CSV import remains available.
1. **Given** a Harvest import is queued or running, **When** the administrator tries to save or remove the client, **Then** the change is refused with guidance to finish or cancel the import first, and nothing changes.
1. **Given** an authorization was started before the client changed, **When** its callback arrives afterwards, **Then** it is rejected and no credentials are stored.

______________________________________________________________________

### User Story 4 - Understand why the client cannot be stored (Priority: P3)

An administrator on a deployment without the encryption key learns why the form cannot save a secret and what the operator has to set.

**Why this priority**: Without the key nothing can be stored securely, so the page must explain the one remaining setup step.

**Independent Test**: Start the server without the encryption key. The importer shows that a client cannot be stored, names the setting the operator must provide, and offers no save action; saving through the server is refused.

**Acceptance Scenarios**:

1. **Given** no encryption key, **When** an administrator opens the importer, **Then** the form explains that the secret cannot be stored until the operator sets the encryption key, names that setting, and cannot be submitted.
1. **Given** no encryption key, **When** a save request reaches the server anyway, **Then** it is refused and nothing is stored.

### Edge Cases

- The encryption key changes after a client was stored: the stored secret can no longer be opened. The page says so and asks for the secret to be entered again; the API import is unavailable until then. Connecting never sends an unreadable or empty secret.
- A save sends a new client ID without a new secret: refused, because a secret belongs to one client. The secret may be left blank only to keep the current one for the same client ID.
- Blank or whitespace-only client ID or secret, or overly long values: refused with a clear message.
- A redirect URL that is not an absolute `https` address ending in the Harvest callback path: refused, since Harvest would reject the authorization or send the code elsewhere. Plain `http` is accepted only for `localhost`, `127.0.0.1` and `[::1]`, so the authorization code never crosses a network unencrypted.
- Two administrators save at the same time: each save is applied whole; the last one wins and no mix of the two is stored.
- A non-administrator, a signed-out visitor, or an administrator of another organization calls the settings actions directly: refused as forbidden or unauthenticated, and nothing about the stored client is revealed.
- The environment supplies the client but not the redirect URL: the API import stays not configured, as today, and the page names the missing redirect URL setting. A stored client, including its redirect URL, is ignored while the environment client is set.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST let an administrator save, replace and remove an organization's Harvest OAuth client (client ID and client secret, saved with the redirect URL shown) from the Harvest importer page.
- **FR-002**: Only active administrators of the organization MAY read the client settings or change them; all other callers MUST be refused as forbidden (or unauthenticated when signed out), and authority MUST be rechecked when the change is written.
- **FR-003**: When both the client ID and the client secret are set in the environment, the system MUST use them, MUST show the client as configured by the environment, read-only, and MUST refuse changes to the stored client for that organization while it is ignored.
- **FR-004**: When the environment does not set both, the system MUST use the organization's stored client. A partial environment client MUST be ignored.
- **FR-005**: The redirect URL from the environment, when set, MUST take precedence and be shown read-only; otherwise the stored redirect URL MUST be used. Otherwise the page MUST show the current Horae address followed by the Harvest callback path, read-only and copyable, and MUST save that value with the client.
- **FR-006**: The client the importer uses (connect, callback, token refresh, queued and running imports) MUST be resolved when it is needed, so saving, replacing or removing it takes effect without a restart.
- **FR-007**: The client secret MUST be stored only in sealed form, with the same authenticated encryption and deployment key that protect the stored Harvest tokens. The key MUST NOT be stored by the system.
- **FR-008**: The client secret MUST NOT be returned by any response, rendered on any page, or written to logs. Responses MUST show only whether a secret is set.
- **FR-009**: When the encryption key is not configured, the system MUST refuse to store a client and the page MUST explain which setting the operator must provide. Removing a stored client MUST still be possible.
- **FR-010**: When the stored secret cannot be opened with the current key, the system MUST treat the API import as not configured and the page MUST ask for the secret again. When a stored client has no redirect URL of its own because the environment supplied one that is no longer set, the page MUST say so and ask for a new save.
- **FR-011**: Saving a different client ID, or removing the client, MUST disconnect Harvest as the existing Disconnect does: tokens removed; account binding, history, reports and imported data retained. Replacing only the secret, or saving again with an unchanged client ID, MUST keep the connection.
- **FR-012**: The page MUST warn before a save or removal that will disconnect Harvest, and the system MUST refuse such a change unless the request confirms it, so a page loaded before Harvest was connected cannot disconnect it unwarned.
- **FR-013**: Every change MUST be refused, without effect, while a Harvest import for the organization is queued or running.
- **FR-014**: Every change MUST invalidate Harvest authorizations started before it, so their callbacks cannot store credentials.
- **FR-015**: Each change MUST record who made it and when, in the organization's audit trail, without the secret.
- **FR-016**: Input MUST be validated: client ID and secret non-blank and of bounded length; redirect URL absolute `https` (or `http` for `localhost`, `127.0.0.1` and `[::1]` only) ending in the Harvest callback path; a new client ID requires a new secret.
- **FR-017**: CSV import MUST remain available whether or not a client is configured.

### Key Entities

- **Harvest OAuth client (stored)**: one per organization. Client ID, sealed client secret, optional redirect URL, who last changed it and when.
- **Effective Harvest client**: what the importer uses for a request: client ID and secret from the environment or the stored client, redirect URL from the environment or the stored client, and the deployment encryption key. Absent when any part is missing or unreadable.
- **Client settings view**: what administrators see: where the client comes from (environment, stored, none), client ID, whether a secret is set, the redirect URL and where it comes from, whether the key is available, whether the stored secret is readable, and whether editing is allowed.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An administrator can go from "not configured" to "ready to connect" on the importer page in under two minutes, with no server restart and no deployment change other than the encryption key.
- **SC-002**: Deployments configured through the environment behave exactly as before: the same client is used and no existing import, connection or test changes outcome.
- **SC-003**: No response, page or log line produced by the system contains a stored client secret, verified against every settings and connection response in the test suite.
- **SC-004**: A stored secret is unreadable without the deployment key: the stored value differs from the secret and cannot be opened with another key.
- **SC-005**: 100% of change attempts by non-administrators, and of changes while an import is queued or running, are refused without changing stored data.

## Assumptions

- The deployment encryption key and the "configured by environment" check are read when the server starts, as environment variables are today; only the stored client changes at runtime.
- Harvest keeps issued tokens valid when the secret of the same client is rotated, so a secret-only change does not need a reconnect. If Harvest rejects a refresh, the existing reconnect flow applies.
- The organization's audit trail is the existing audit log; there is no page for it yet, so recording the change there is sufficient.
- The command-line import commands talk to the running server and are unaffected; they need no client settings of their own.
- No separate public-URL setting exists; the shown redirect URL comes from the address the administrator is using. Deployments reached under several addresses can pin it with the existing environment setting.
