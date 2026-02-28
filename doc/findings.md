# Code Review Findings

Date: 2026-02-27

## Brief Summary

A Rust REST API service (`waddle-ws`) for managing financial portfolio
transactions. Users upload a semicolon-delimited CSV of buy/sell
transactions via `PUT /transactions`, retrieve them as JSON via
`GET /transactions`, and get an aggregated portfolio (symbol → quantity
map) as YAML via `GET /portfolio`. State is held in-memory with no
persistence.

## Findings

### transactions.rs

1. **Opaque error type in `try_from_reader`** — Returns `csv::Result`
   with no diagnostic context. Return `anyhow::Result` and add
   `.context(...)` calls for diagnosable errors.

### ws.rs

1. **`env::var_os` unwrap** — `into_string().unwrap()` panics on
   non-UTF-8 `OsString`. Use `env::var("WADDLE_LISTEN_ADDR")
   .unwrap_or_else(|_| "127.0.0.1:3000".to_string())`.

1. **Bare `.unwrap()` in `main`** — `.unwrap()` calls on `bind` and
   `serve` produce undiagnosable panics. Use `.expect("descriptive
   message")` or propagate with `?` via a `run()` function.

1. **Mutex poison panic in handlers** — `lock().unwrap()` in
   `get_portfolio` and `get_transactions` causes an unrecoverable panic
   if the mutex is poisoned. Match on `PoisonError` and return `500`
   instead. (`upload_transactions` already handles this correctly.)

1. **Multipart error value discarded** — The closure in `next_field`
   uses `|_|`; a generic message is logged but the actual error is
   lost. Change to `|e|` and include `e` in the `error!` call.

1. **Bytes read error discarded** — `f.bytes().await.map_err(|_|…)`
   silently drops the underlying I/O error. Log it before mapping to a
   status code.

1. **`t.iter().count()` for logging** — Walks the entire vec just to
   log the count. Expose a `Transactions::len()` method and call that.

1. **Silent data replacement** — Every `PUT` silently overwrites
   existing data. Document as a known limitation or require a
   confirmation header.

1. **No upload size limit** — A very large CSV is buffered entirely in
   memory. Add an Axum `DefaultBodyLimit` layer to bound memory use.

## Scorecard

| Criterion              | Orig | Now  | Notes                               |
| ---------------------- | ---- | ---- | ----------------------------------- |
| Clarity of Intent      | 7/10 | 8/10 | `list_trans`, `my_days_iter`, typo  |
| Architecture & SOLID   | 5/10 | 6/10 | Vec choice documented by design     |
| Technical Solutions    | 6/10 | 6/10 | Error handling, body-limit open     |
| Clean Code & Naming    | 5/10 | 7/10 | Dead attrs, `my_` prefix resolved   |
| Testability & Security | 6/10 | 8/10 | All six test antipatterns resolved  |

## Golden Rule

Add `.context(...)` to all error paths in `try_from_reader` and return
`anyhow::Result` so every parse failure carries a diagnostic message.
The `csv::Result` type gives no call-site context when errors surface
in the REST handler — switching to `anyhow` costs one dependency line
and makes every failure self-diagnosing.
