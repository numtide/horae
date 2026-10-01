# Project archive and expense correction

Observed 2026-10-01 as account owner using only the disposable project and
expense. Category archive is covered separately in [category-evidence.md](category-evidence.md).

## Project archive

Archiving the fixture through its project Actions menu retained its expense and
receipt. The expense became locked with an archive-related reason. Its editor
disabled date, project and category; notes, quantity, receipt and billability
remained enabled. An owner UI save changed quantity from 3 to 4 and total from
USD 1.50 to USD 2.00. A separate form-endpoint correction restored quantity 3 /
USD 1.50 and changed billable to false, returning 200 while the archive lock
remained. This differs from the invoiced-source editor, which disables
billability. Do not use one blanket field policy for every lock reason.

New expense capture against that archived project returned 422 with a person /
project assignment error and no new identity. The existing receipt remained
downloadable, returning 200 and the original 431 bytes. These owner results do
not establish non-owner permissions or receipt access after grant revocation.

The detailed report filtered to the fixture project included its USD 1.50 by
default. Checking Active projects only added `active_projects=true` and showed
no matching expenses. Thus archived history and the active-only filter are
distinct from the archived-choice option in the filter picker.

Restoring the project through its Restore button brought back Edit project and
removed the archive banner. Expense baseline restoration is tracked in the
private fixture ledger. No business record, person, invoice state or workspace
lock was changed during this experiment.

## Shared contract consequence

FR-006 must distinguish invoice and archive locks. Feature 015 owns who can
perform these privileged corrections; an owner success is not evidence that
every expense-write grant permits them. Multiple simultaneous locks must still
be evaluated independently rather than letting one successful correction bypass
another protection.

The [unlocking guide](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses)
documents distinct invoice, approval, archive and administrative reasons. The
[detailed report guide](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports)
documents archived-history inclusion. The field-level results above come from
the isolated live experiment, not an inference from those guides.
