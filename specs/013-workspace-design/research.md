# Workspace source and behavior research

Status: specification input, not completed planning research. Baseline `9301112`.

## Current application

- `components/admin_shell.rs` is administrator-only, with real People and Harvest Importers destinations. Its source explicitly defers the other Workspace destinations.
- `pages/admin.rs` already creates users and changes their role/activation. `server_fns/users.rs` preserves the last active administrator under concurrent role/activation changes. Creating an account is not an email invitation.
- `list_users` has its own visibility/rate policy. The handoff's simple matrix does not capture every manager/project-assignment condition; the new matrix must describe current authority rather than rewrite it.
- `organizations` has name, default currency, first weekday and rounding settings, plus invoice branding added by later migrations. No persisted workspace timezone or approval-policy flag was found. Currency-less person rates currently depend on organization currency, so a default-currency edit cannot just relabel them.
- `server_fns/invoices.rs` and `invoices/entries.rs` currently permit eligible open/approved time and reject submitted time. The handoff's enabled approval toggle is not an existing setting or a safe migration default.
- `audit_log` exists in the initial migration, but no current administrative insertion path was found by repository search. Merely exposing the table cannot substantiate the handoff's claim to cover every administrative action.
- Durable import jobs and private report artifacts already exist. They are reusable infrastructure, not evidence of scheduled workspace backups or a tested restore workflow. Mail transport configuration exists for other features, not proof that invitations are implemented.

## Handoff findings

The complete `09_Workspace.dc.html` source defines five sections: People, Roles & permissions, General, Export & backups, Audit log. It links `03_Invite Team.dc.html`, which includes per-email roles and a reusable share link. Those invitation buttons do not implement delivery/admission.

Other design contracts requiring reconciliation:

- People includes last-active dates and seat counts; neither should be fabricated from user creation dates or commercial example quotas.
- General includes an editable hosted URL, a timezone picker, Monday/Sunday selection, currency and an approval toggle. Existing valid weekdays include all seven days, and the self-hosted deployment has no tenant slug.
- Delete redirects to a sign-in prototype. Its dialog promises exports remain downloadable for 30 days without defining who can authenticate after deletion.
- Full export says one CSV per entity in a ZIP. Automatic backup says Monday 03:00, retained 90 days. Download latest only opens a toast; it is not evidence of storage or recovery.
- Audit heading says 12 months, while Load more opens a toast claiming 90 days. Neither arbitrary prototype count nor incompatible retention copy is authoritative production evidence.

## Harvest reference

On 2026-09-30 the existing approved Windows Chrome MCP connection navigated, read-only, to `/company/account` and `/company/settings/preferences`. The former is Billing, with navigation to Preferences, Modules, Sign in security, Import/Export and Bulk actions. Preferences presents a summary with an Edit preferences action.

Observed categories include timezone/week start, weekly capacity, timesheet reminders/deadlines, rounding, date/time presentation, default currency, approval-related policies and project budget rules. No settings were saved and no records deleted. This was DOM/reference inspection, not native editing or persistence acceptance. Local evidence is retained under Horae's ignored `.scratch/playwright-windows/`.

These observations do not authorize Harvest commercial billing, new authentication, generalized self-join or a changed permission model. Horae's handoff and reconciled specification remain the implementation contract.

## Specification workflow

- Followed the repository's `speckit-specify` skill and active local spec template. No preset overrides or extension hooks are configured; no standalone `specify` executable was found on the current shell path.
- Reserved feature 013 after inspecting the existing specs and the separate feature 012 Clients worktree. The branch is independent of the feature-directory name.
- The constitution governs money/time exactness, organization isolation, existing authenticated mutations and reproducible verification.
- FR-006, FR-014 and FR-019 remain unresolved. No plan/tasks or completed-clarification claim is made. Personal Settings and the other delivery surfaces remain open.
