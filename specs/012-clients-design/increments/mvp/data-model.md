# Clients MVP data model

No new persisted entities or migrations.

| Entity/projection | Fields and constraints |
| --- | --- |
| Client | Existing UUID v7, org ID, name, ISO currency, optional address/tax ID, active flag, creation time. Catalog authority unchanged. |
| Billing details | Optional default-rate cents, manager/admin only. Unauthorized section differs from authorized unset rate; unset differs from zero. |
| Summary | Client plus active/total authorized project counts and distinct visible project currencies. Hidden projects cannot influence search/counts. |
| Related projects | Current progress projection with `project_read_access` and existing rate redaction/aggregate-budget policy. |
| Related invoices | Manager-only org/client-scoped stored rows with status/amount/currency. No mixed-currency total. |
| Form | Name/currency/address/tax ID plus explicit keep/replace/clear rate intent and edited original denomination/rate for conflict detection. |
| Route context | Optional client ID proposed once to existing editor, not a draft or new business record. |

Reuse existing validation: trimmed name 1–200 characters, no NUL; supported
project currencies; optional nonnegative representable decimal parsed to cents.
Preserve legitimate address/tax-ID text, reject NUL, consistently normalize empty
optional text to absent. No new contact validation.

Session determines actor/org. Updates lock the client and compare original
currency/rate before explicit replacement to reject stale financial edits.
No-op saves emit no update event; events follow commit. Lifecycle writes affect
only the client flag. No client save rewrites historical project/invoice money,
currency, terms or associations; defaults flow only through future creation.
