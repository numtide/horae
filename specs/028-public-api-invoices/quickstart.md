# Quickstart: importing invoices into the books

On the dev stack (`process-compose up`), signed in as Admin at http://localhost:8080.

1. Personal settings → API tokens → create one named "Books", and copy it:

   ```sh
   export TOKEN=horae_pat_…
   api() { curl -fsS -H "Authorization: Bearer $TOKEN" "http://localhost:8080/api/v1$1" "${@:2}"; }
   ```

1. Issue an invoice in the web app (Invoices → New, then Mark as sent).

1. List the issued invoices and read one:

   ```sh
   api '/invoices?status=sent' | jq '.data[] | {id, number, client_name, total, issued_on}'
   id=$(api '/invoices?status=sent' | jq -r '.data[0].id')
   api "/invoices/$id" | jq '.lines'
   ```

1. Download its PDF; it is the web app's download, byte for byte:

   ```sh
   api "/invoices/$id/pdf" -o invoice.pdf
   ```

1. Incremental sync: remember when a run starts, and next time ask only for what changed since.
   Mark the invoice paid in the web app, then:

   ```sh
   api "/invoices?updated_since=$(date -u -d '-10 min' +%Y-%m-%dT%H:%M:%SZ)" | jq '.data[] | {number, status}'
   ```

   Follow `next_cursor` (`&cursor=…`, with the same filters) until it is `null`.

1. Refusals: a token of a Member gets 403 `forbidden` on all three; an OAuth app's token gets 403
   `insufficient_scope`; an unknown id gets 404 `not_found`.
