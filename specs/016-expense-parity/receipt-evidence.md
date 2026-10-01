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

## Content and format follow-up

The next fixture's existing 431-byte PDF was downloaded and checked before
replacement. Benign synthetic text and one-pixel images exercised the same
multipart endpoint; each attempt was followed by a receipt download.

| File / declared MIME | Result | Download after attempt |
| --- | --- | --- |
| Empty PDF / application/pdf | 422 content mismatch | Previous PDF retained |
| Plain text named .pdf / application/pdf | 422 content mismatch | Previous PDF retained |
| Plain text named .png / image/png | 422 content mismatch | Previous PDF retained |
| PNG, 68 bytes / image/png | 200 | PNG, same size |
| Same PNG named .png / text/plain | 200 | Served as image/png |
| Same PNG named .txt / image/png | 422 content mismatch | Previous PNG retained |
| GIF, 34 bytes / image/gif | 200 | GIF, same size |
| Canvas-generated JPEG, 285 bytes, .jpg / image/jpeg | 200 | JPEG, same size |
| Same JPEG, .jpeg / image/jpeg | 200 | JPEG, same size |

These cases establish content-sensitive validation and MIME normalization, not
an extension-only or client-MIME-only rule. They do not prove full structural
decoding, antivirus scanning or sanitization of every accepted document. No
executable payload was uploaded. The original synthetic PDF, note and quantity
were restored successfully; download returned 200 with the original 431 bytes.
Private results: `expense-independent-results-20261001.md`.

## Recognizable malformed files

A nine-byte file consisting only of `%PDF-1.4` and a newline, named `.pdf` and
declared application/pdf, was accepted and downloaded unchanged as a PDF. It
contains no page tree, content or cross-reference table. An eight-byte PNG
signature without image data, named `.png` and declared image/png, was rejected
with a content-mismatch error; the previous nine-byte PDF remained downloadable.
The original 431-byte valid synthetic PDF was then restored and downloaded.

Therefore acceptance is **not full document-validity certification**. Preserve
the distinction between upload type checks and render/preview failure handling;
do not claim every accepted PDF is renderable. Do not add a stricter structural
rejection rule and call it verified Harvest parity. Security controls remain
required independently of this observed weak reference validation.

Remaining checks: non-owner access/revocation, storage/backup retention and
invoice-recipient artifact lifecycle. Provider backup erasure is not observable
through this browser; the Horae storage policy needs its own explicit contract.
Private cases: `expense-lifecycle-results-20261001.md`. These tests do not close
the full attachment contract.

## Internal download versus generated-report rendering

Read-only follow-up on the restored valid fixture found a list control labelled
Download attachment, opening the internal receipt route in a new tab. Its
response was `application/pdf` with `Content-Disposition: attachment` and the
expected 431 bytes. The editor displayed Attached receipt / Delete receipt,
not an embedded PDF preview; its one hidden iframe belonged to an existing
payment-library bootstrap, not the receipt. Cancelled the editor without saving.

Do not invent an in-app PDF viewer or claim that this download path validates
rendering. The earlier accepted header-only PDF likewise remained downloadable.
Failures while incorporating receipts into a generated report are a different
contract: export generation may email, and the current invoice Preview has its
separately recorded access gate. Those artifacts cannot be certified from a
successful internal receipt download. No malformed file was uploaded again
merely to repeat its already-proven acceptance.
