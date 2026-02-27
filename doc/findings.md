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

1. **`list_trans` (line 13)** — Shadows the `p` path parameter with a
   local `Portfolio` binding. Rename param to `path`, or remove the
   function if unused.

1. **`list_trans` double iteration (lines 20–21)** — Iterates `t` twice:
   once for printing, once to build the portfolio. Combine into a single
   pass.

1. **`Portfolio.data` wrong data structure (line 29)** — `Vec<(String, i32)>`
   requires an O(n) linear scan on every `add_transaction`, `amount`, and
   `symbol_iter` call. Use `IndexMap<String, i32>` or `HashMap` instead.

1. **Redundant O(n) scans (lines 58–63)** — `amount()` and
   `add_transaction()` each independently scan the vec. Backing with a map
   eliminates both; sort to a vec only at serialisation time.

1. **`commision` typo (line 106)** — Field name and CSV header were
   misspelled. **Fixed in commit `99730d9`** — renamed to `commission`.

1. **Opaque error type in `try_from_reader` (line 143)** — Returns
   `csv::Result` with no diagnostic context. Return `anyhow::Result` and
   add `.context(...)` calls for diagnosable errors.

1. **`my_days_iter` (line 169)** — Non-idiomatic `my_` prefix; boxes an
   iterator unnecessarily. Rename to `days_iter` and return a concrete
   type.

1. **`#[allow(dead_code)]` on public `iter()` (line 176)** — Public API
   is not dead code. Remove the attribute.

1. **Two linear scans for date range (lines 182–188)** — `first_date` and
   `last_date` each do a full scan and are called together in `my_days_iter`.
   Compute both in a single `fold`.

1. **`#[allow(dead_code)]` on `Portfolio` struct (line 87)** — The struct
   has a private `data` field; the allow-attribute suppresses all item-level
   warnings. Remove it and warn per-item only if needed.

### ws.rs

1. **`env::var_os` unwrap (line 39)** — `into_string().unwrap()` panics
   on non-UTF-8 `OsString`. Use `env::var("WADDLE_LISTEN_ADDR")
   .unwrap_or_else(|_| "127.0.0.1:3000".to_string())`.

1. **Bare `.unwrap()` in `main` (lines 48–50)** — Three `.unwrap()` calls
   produce undiagnosable panics. Use `.expect("descriptive message")` or
   propagate with `?` via a `run()` function.

1. **Mutex poison panic in handler (line 54)** — `lock().unwrap()` in an
   async handler causes an unrecoverable panic if the mutex is poisoned.
   Match on `PoisonError` and return `500` instead.

1. **Errors silently discarded on multipart failure (line 84)** — The
   original error is dropped with `map_err(|_|)`. Log the error before
   mapping to a status code.

1. **Bytes read error discarded (line 94)** — Same issue as above; log
   before mapping.

1. **`t.iter().count()` for logging (line 100)** — Walks the entire vec
   just to log the count. Expose a `Transactions::len()` method and call
   that.

1. **Silent data replacement (lines 100–106)** — Every `PUT` silently
   overwrites existing data. Document as a known limitation or require a
   confirmation header.

1. **No upload size limit** — A very large CSV is buffered entirely in
   memory. Add an Axum `DefaultBodyLimit` layer to bound memory use.

## Scorecard

- **Clarity of Intent: 7/10** — Goal is readable overall; `list_trans`,
  `my_days_iter`, and the typo reduced clarity.
- **Architecture & SOLID: 5/10** — `Transactions` handles parsing,
  iteration, date logic, and acts as a repository — SRP violated;
  `Portfolio` mixes storage and display; no trait abstractions.
- **Technical Solutions: 6/10** — Good library choices; `serde_yaml 0.9`
  is unmaintained (use `serde_yml`); `Vec` is the wrong structure for the
  portfolio.
- **Clean Code & Naming: 5/10** — `my_` prefix, the typo (fixed), unnamed
  tuple fields, `p` shadowed by a same-named parameter.
- **Testability & Security: 6/10** — Good unit test coverage; no upload
  size limit is a DoS risk; mutex poison panic in handler is a reliability
  risk; no authentication.

## Golden Rule

Replace `Vec<(String, i32)>` in `Portfolio` with `IndexMap<String, i32>`.
This eliminates the O(n) scan on every transaction insert and every
`amount()` call, and preserves insertion order for deterministic YAML
output — all without changing the public API surface.
