# Personal Settings source audit

Status: input to specification, not completed implementation planning. Baseline `9301112`.

## Current behavior

- `pages/settings.rs` implements device-local theme selection and installed-plugin information. Its existing behavior must survive the redesigned page.
- `server_fns/auth.rs` exposes the current user and current-session logout. There is no personal-profile mutation in that module.
- Users persist a full name, email, identity-provider subject, role, rates and active status. The inspected schema has no first/last name, profile photo, personal timezone, notification preferences or manager-to-person relation.
- `auth/oidc.rs::resolve_user` links an unbound active account through verified email and subsequently resolves by subject. It does not synchronize names/photos on each login. Profile editing must not rebind that subject or turn an arbitrary email edit into identity proof.
- `server_fns/users.rs` changes role and activation through protected administrative operations; ordinary personal settings cannot become a second role-edit route. Rate visibility and project-specific access need to be checked against their actual current consumers.
- `server_fns/approvals.rs` uses organization-manager authority. Shared project membership does not itself define a manager-to-person relationship.
- Project budget notifications already have an outbox and mail prerequisites. That is reusable infrastructure, not proof that timesheet reminders, weekly digests or the other personal notification categories exist.
- `route.rs` redirects the root to the timesheet. No welcome-home destination was found.

## Complete handoff source findings

`08_Settings.dc.html` defines Basic info, Assigned people, Permissions and Notifications. Rates, Assigned projects, Integrations and Security are explicitly rendered through a generic “Nothing to configure here yet in this mockup” section.

- Basic info contains first/last name, disabled work email, a rates link, timezone, photo replacement/removal and welcome-home preference. Update info has no persistence handler.
- Permissions mixes six Harvest-like profiles with a static warning that Manager is unavailable. This conflicts with Horae's fixed organization roles and its project-specific authority; none of those warnings/profiles should be copied as live facts.
- Assigned people shows a static empty state and assumes per-person management that the current schema does not represent.
- Notifications lists daily personal reminders, automatic team reminders, weekly time reports, people/project submission emails, managed-project deletion notices and promotional mail. The Sunday 23:00 deadline is example text, not current workspace configuration.
- Security's description mentions password, sessions and two-factor authentication but contains no defined workflow. Integrations and Rates are similarly placeholders, not permission to introduce new provider/financial products without a contract.

## Specification workflow and gates

- Used the checked-in `speckit-specify` instructions, local spec template and constitution. No template preset overrides or extension hooks are configured.
- Feature 014 follows Clients 012 and Workspace 013, each on its own branch/worktree. `.specify/feature.json` points to this feature independently of branch naming.
- FR-003/007/010 require product decisions. Proposed treatment of placeholder home/security/marketing/rate behavior also needs acceptance; it is not a completed design deviation.
- No application, schema, deployment, auth policy or real user data changes are part of this draft. Personal Settings remains required by the wider goal.
