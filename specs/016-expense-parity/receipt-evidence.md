# Persisted receipt boundary evidence

Checked 2026-10-01 in authenticated Harvest using the authorized disposable
expense. Synthetic blank PDFs contained a catalog, page tree, one blank page,
cross-reference table and end marker; comment padding before the objects varied
the size without changing the structure. No business document was uploaded.

Tests submitted multipart files through the observed expense form endpoint from
the same browser session. Tokens stayed in the browser. These results establish
server acceptance, not file-picker interaction or non-owner access.

| PDF bytes | HTTP result | Persisted file size |
| --- | --- | --- |
| 431 | 200 | 431 |
| 10,000,000 | 200 | 10,000,000 |
| 10,000,001 | 200 | 10,000,001 |
| 10,485,759 | 200 | 10,485,759 |
| 10,485,760 | 422 | Previous receipt retained |
| 10,485,761 | 422 | Previous receipt retained |

The exact accepted maximum is **10,485,759 bytes**: the limit is strictly less
than 10 MiB, not 10 decimal MB or an inclusive 10 MiB. The boundary error states
that the attachment must be less than 10 megabytes.

Replacing the largest accepted PDF with an exactly 10 MiB PDF while changing the
expense note returned 422. Downloading the existing receipt still returned 200
and 10,485,759 bytes; reloading the expense preserved the original note and USD
1.00 total. The failed replacement did not partially save that note.

A subsequent plain-text `.txt` replacement returned 422 with the allowed list
`.png`, `.gif`, `.pdf`, `.jpg`, `.jpeg`. This does not prove content sniffing,
acceptance of every listed format, or a malformed file's handling. The initial
input advertised corresponding image/PDF MIME types, including `application/x-pdf`.
Reloading also preserved the original note, quantity 2 and total USD 1.00 rather
than the deliberately changed note and quantity 3 submitted with the rejected file.

In the browser form, choosing Delete receipt and then Cancel retained the
downloadable 10,485,759-byte PDF after reload. Choosing Delete receipt and then
Update expense removed the link while preserving the expense and total; the old
expense receipt route returned 404 with caching disabled. Only the synthetic
test receipt was removed. This verifies application-route removal, not deletion
from backups, remote storage or an already-generated invoice attachment.

Evidence is local to `.scratch/playwright-windows/`, including
`expense-receipt-boundary-readback-20261001.md` and
`expense-receipt-removed-20261001.md`; exact fixture identifiers and
cleanup state are in the private ledger. Files and authenticated receipt links
are not committed or shared externally.

Remaining checks: content validation, non-owner access/revocation,
storage/backup retention and invoice-recipient artifact
lifecycle. These boundary tests do not close the full attachment contract.
