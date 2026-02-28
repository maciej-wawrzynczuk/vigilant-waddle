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

1. ~~**`list_trans` (line 13)** — Shadows the `p` path parameter with a
   local `Portfolio` binding.~~
   **Resolved `45f3c94`** — function removed.

1. ~~**`list_trans` double iteration (lines 20–21)** — Iterates `t`
   twice: once for printing, once to build the portfolio.~~
   **Resolved `45f3c94`** — function removed.

1. **`Portfolio.data` wrong data structure** — `Vec<(String, i32)>`
   requires an O(n) linear scan on every `add_transaction` call. Use
   `IndexMap<String, i32>` or `HashMap` instead.

1. ~~**Redundant O(n) scans** — `amount()` and `add_transaction()` each
   independently scan the vec.~~
   **Partially resolved `45f3c94`** — `amount()` removed; `add_transaction`
   still does an O(n) scan (see item above).

1. ~~**`commision` typo** — Field name and CSV header misspelled.~~
   **Resolved `99730d9`** — renamed to `commission` throughout.

1. **Opaque error type in `try_from_reader`** — Returns `csv::Result`
   with no diagnostic context. Return `anyhow::Result` and add
   `.context(...)` calls for diagnosable errors.

1. ~~**`my_days_iter`** — Non-idiomatic `my_` prefix; boxes an iterator
   unnecessarily.~~
   **Resolved `45f3c94`** — method removed (no production caller).

1. ~~**`#[allow(dead_code)]` on public `iter()`** — Public API is not
   dead code. Remove the attribute.~~
   **Resolved `45f3c94`** — attribute removed.

1. ~~**Two linear scans for date range** — `first_date` and `last_date`
   each do a full scan and are called together.~~
   **Resolved `45f3c94`** — both methods removed (no production caller).

1. ~~**`#[allow(dead_code)]` on `Portfolio` struct** — Suppresses all
   item-level warnings indiscriminately.~~
   **Resolved `45f3c94`** — attribute removed.

### ws.rs

1. **`env::var_os` unwrap** — `into_string().unwrap()` panics on
   non-UTF-8 `OsString`. Use `env::var("WADDLE_LISTEN_ADDR")
   .unwrap_or_else(|_| "127.0.0.1:3000".to_string())`.

1. **Bare `.unwrap()` in `main`** — Three `.unwrap()` calls produce
   undiagnosable panics. Use `.expect("descriptive message")` or
   propagate with `?` via a `run()` function.

1. **Mutex poison panic in handler** — `lock().unwrap()` in an async
   handler causes an unrecoverable panic if the mutex is poisoned.
   Match on `PoisonError` and return `500` instead.

1. **Errors silently discarded on multipart failure** — The original
   error is dropped with `map_err(|_|)`. Log the error before mapping
   to a status code.

1. **Bytes read error discarded** — Same issue as above; log before
   mapping.

1. **`t.iter().count()` for logging** — Walks the entire vec just to
   log the count. Expose a `Transactions::len()` method and call that.

1. **Silent data replacement** — Every `PUT` silently overwrites
   existing data. Document as a known limitation or require a
   confirmation header.

1. **No upload size limit** — A very large CSV is buffered entirely in
   memory. Add an Axum `DefaultBodyLimit` layer to bound memory use.

### Tests

1. ~~**`test_from_csv` discards parsed result** — No field assertions;
   only checks parsing does not panic.~~
   **Resolved `8619852`** — now asserts `symbol`, `number`, `price`.

1. ~~**Portfolio tests depend on CSV parser** — Coupling unit tests to
   `try_from_reader` means a CSV bug breaks portfolio tests.~~
   **Resolved `8619852`** — portfolio tests construct `MyTransaction`
   directly via `make_tx` helper.

1. ~~**Multipart body construction duplicated** — Four tests copy-paste
   the same three-line body construction.~~
   **Resolved `8619852`** — extracted `multipart_csv` helper.

1. ~~**Upload response discarded with `let _`** — A failed upload causes
   a misleading downstream failure.~~
   **Resolved `8619852`** — upload status is now asserted before the
   GET.

1. ~~**Internal state inspection via `Arc<Mutex<...>>`** — Tests reach
   into private storage instead of verifying observable behaviour.~~
   **Resolved `8619852`** — replaced with `GET /transactions` assertions.

1. ~~**`AppState` initialisation duplicated** — Identical three-line
   block repeated in all six tests.~~
   **Resolved `8619852`** — extracted `empty_state` helper.

## Scorecard

|Criterion|Original|Current|Notes|
|---|---|---|---|
|Clarity of Intent|7/10|8/10|`list_trans`, `my_days_iter`, typo all gone|
|Architecture & SOLID|5/10|5/10|SRP and Vec structure still unresolved|
|Technical Solutions|6/10|6/10|`serde_yaml 0.9` and Vec still open|
|Clean Code & Naming|5/10|7/10|Dead attributes, `my_` prefix, typo resolved|
|Testability & Security|6/10|8/10|All six test antipatterns resolved|

## Golden Rule

Replace `Vec<(String, i32)>` in `Portfolio` with `IndexMap<String, i32>`.
`add_transaction` still performs an O(n) scan on every insert, and the
tuple representation has no named fields. A map eliminates the scan and
preserves insertion order for deterministic YAML output — all without
changing the public API surface.
