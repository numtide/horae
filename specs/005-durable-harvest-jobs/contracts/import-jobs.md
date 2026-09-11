# Import Jobs Contract

All operations are authenticated server functions and restricted to organization administrators.

| Operation | Input | Result |
|---|---|---|
| `start_harvest_api_import` | mode, sync scope | job ID + initial status |
| `start_harvest_csv_import` | mode, upload | job ID + initial status |
| `get_import_job` | job ID | status, progress, report/error |
| `list_import_jobs` | optional limit/cursor | recent jobs for current organization |
| `cancel_import_job` | job ID | updated status |
| `retry_import_job` | job ID | new attempt/status, subject to policy |

The contract never returns credentials or raw CSV contents. Status reads must reject foreign-organization job IDs as not found or forbidden according to the existing server-function convention.
