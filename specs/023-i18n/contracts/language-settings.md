# Contract (draft): language settings

**Status**: draft for the later increments ([plan.md](../plan.md) *Later increments*). The first
increment changes no server function, no `horae-types` type and no HTTP response. This file
records the interface those increments start from; their own plans may refine it.

## Unchanged by the whole feature

- `/api/v1`: no response depends on `Accept-Language` or on a person's or organization's
  language (FR-042, SC-009). The `api-compat` and `api-v1` checks pass unchanged, and the OpenAPI
  document gains no language field.
- `/harvest/v2`: the same rule (FR-042).
- CSV and XLSX exports: headers and fixed values in English, byte for byte (C-4, FR-052, SC-007).

## Server functions (choosing the language)

| Function | Change | Notes |
| --- | --- | --- |
| `get_me` → `CurrentUser` | adds `language: Option<String>` (the person's stored tag) and `workspace_language: Option<String>` | Same pattern as `timezone` and `workspace_timezone`; the server returns only tags that `Lang::from_tag` knows |
| new `get_interface_language() -> String` | the resolved tag for this request: person, organization, `Accept-Language`, English | Read by `App` with `use_loader` before the first render (FR-033); works without a session (it then skips the first two steps) |
| `update_own_profile` | takes `language: Option<String>`; `None` clears the preference | Calls `ops::profile::update`, beside the time zone; an unknown tag is an `OpError::invalid("language", …)` |
| `ProfileOptions` | adds `languages: Vec<(String, String)>`, each tag with its name in its own language | Lists only `Lang::ALL` (FR-035) |
| new `set_workspace_language(language: Option<String>)` | sets or clears the organization's default | Beside `set_workspace_timezone`; a new `ops::organization` operation under `Permission::CompanyWrite`, like `set_timezone` (FR-031) |
| `update_org_branding` | takes `invoice_language: String` | `ops::organization::update_branding`, `Permission::InvoiceWriteAll` (FR-050) |

## HTML

- The server writes `<html lang="<tag>">` with the resolved tag on every Dioxus page and on the
  sign-in page (FR-033, FR-034). The mechanism is that increment's first research item.
- The hydrated page uses the tag serialized by `use_loader`, so no text changes after hydration
  (SC-006).

## Server messages increment

- `ServerFnError` details carry `reason` and named `args` for each user-visible operation error;
  `message` keeps the English text. The screen formats the catalog message for the reason, and
  shows `message` when the reason has none in the reader's language (FR-041).
- `/api/v1` problem documents keep their current `detail`; adding a field there would be a
  contract change and is not part of this feature.
