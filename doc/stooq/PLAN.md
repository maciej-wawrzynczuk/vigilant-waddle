<!-- markdownlint-disable MD013 MD024 -->
# PLAN: Stooq Quote Provider

## Feature 1 — Async `Quotes` Trait

### Target State

`Quote` carries only `price: Decimal`. `Quotes` is an `#[async_trait]` with
`async fn price`. `MockQuotes` stores prices only and its `insert` takes
`(symbol, price)`. `Send + Sync` become supertraits on `Quotes`, removing
the need to write `Arc<dyn Quotes + Send + Sync>` at call sites.

### Architecture

`async-trait` is added as a dependency. The `#[async_trait]` macro rewrites
`async fn` methods into boxed futures compatible with `dyn` dispatch.
No other architectural change: `MockQuotes` stays in `src/transactions.rs`.

### File Manifest

| File | Change |
| --- | --- |
| `Cargo.toml` | Add `async-trait`, `reqwest`, `toml` |
| `src/transactions.rs` | Update `Quote`, `Quotes`, `MockQuotes` |

### WBS

1. Add to `Cargo.toml` under `[dependencies]`:
   - `async-trait = "0.1"`
   - `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }`
   - `toml = "0.8"`
2. In `src/transactions.rs`, remove `currency: String` from `Quote`.
3. Add `use async_trait::async_trait;` import.
4. Annotate `Quotes` trait with `#[async_trait]`; add `Send + Sync` supertraits;
   change `fn price` to `async fn price`.
5. Change `MockQuotes.data` from `HashMap<String, Quote>` to `HashMap<String, Decimal>`.
6. Update `MockQuotes::insert` signature to `(symbol: impl Into<String>, price: Decimal)`.
7. Update `MockQuotes::price` to `async`; default fallback returns `Quote { price: Decimal::ONE }`.
8. Remove unused `Arc` import if no longer needed in `transactions.rs`.

### Verification

```bash
cargo check
```

No compile errors; no unused import warnings.

---

## Feature 2 — `Portfolio` Carries Currency; `Portfolio::valuation()`

### Target State

`Portfolio.data: Vec<(String, i32, String)>` stores `(symbol, qty, currency)`.
Currency is taken from the first transaction for each symbol. `Portfolio` has no
`quotes` field. The custom `Serialize` impl is deleted. A new
`PortfolioValuation(Vec<PortfolioEntry>)` type derives `Serialize` and is
returned by the new `async fn valuation(&self, quotes: &dyn Quotes)` method.

### Architecture

`valuation()` iterates `self.data`, calls `quotes.price(symbol).await` for each
holding, computes `Decimal::from(qty) * price`, converts to string, and
collects into `Vec<PortfolioEntry>`. Errors propagate via `?`. The method
signature requires `#[async_trait]` is **not** needed here because `Portfolio`
itself is not a trait implementor — `valuation` is a plain `async fn` on the
struct.

`PortfolioValuation` uses `#[serde(transparent)]` to serialize as a bare JSON
array, preserving the existing `GET /portfolio` response shape.

### File Manifest

| File | Change |
| --- | --- |
| `src/transactions.rs` | Refactor `Portfolio`; add `PortfolioValuation`, `PortfolioEntry` |

### WBS

1. Change `Portfolio.data` to `Vec<(String, i32, String)>`; remove `quotes` field.
2. Rename `from_transactions_with_quotes` → `from_transactions(t: &Transactions) -> Self`.
3. In `from_transactions`, extract currency from `tx.currency` for each new symbol;
   accumulate quantity for existing symbols (currency from first occurrence).
4. Delete the custom `impl Serialize for Portfolio` block.
5. Delete the private `PortfolioEntryView` struct.
6. Add public `PortfolioEntry { symbol: String, quantity: i32, value: String, currency: String }`
   deriving `Serialize`.
7. Add `pub struct PortfolioValuation(Vec<PortfolioEntry>)` with
   `#[derive(Serialize)]` and `#[serde(transparent)]`.
8. Add `pub async fn valuation(&self, quotes: &dyn Quotes) -> anyhow::Result<PortfolioValuation>`:
   - Iterate `&self.data`
   - `let quote = quotes.price(symbol).await?;`
   - `let value = (Decimal::from(*qty) * quote.price).to_string();`
   - Push `PortfolioEntry { symbol: symbol.clone(), quantity: *qty, value, currency: currency.clone() }`
   - Return `Ok(PortfolioValuation(entries))`
9. Update unit tests in the `test` module:
   - `mock_quotes_known_symbol`: remove `currency` assertion; update `insert` call to
     `q.insert("FOO", "5.00".parse().unwrap())`
   - `mock_quotes_unknown_symbol`: remove `currency` assertion
   - `portfolio_values_computed`: make `#[tokio::test]`, call `p.valuation(&q).await`,
     currency in JSON now comes from CSV (`"BAR"` — what `make_transactions` writes)
   - `portfolio_multi_currency_short`: make `#[tokio::test]`; write a raw CSV string
     with `USD` and `EUR` per-symbol currency columns so the test can still verify
     that distinct currencies flow through correctly; e.g.:
     `"date;symbol;number;price;commission;currency\n2000-01-01;FOO;3;1.00;0;USD\n2000-01-01;BAR;-1;1.00;0;EUR\n"`
   - `portfolio_zero_quantity` (new, per spec AC 11): make `#[tokio::test]`; test that
     zero quantity produces `value: "0"` and currency from CSV
   - `make_transactions`: keep as-is (all rows use `"BAR"` currency)

### Verification

```bash
cargo test --lib transactions::
```

All tests pass; no clippy warnings in `transactions.rs`.

---

## Feature 3 — `StooqQuotes` Provider

### Target State

New file `src/stooq.rs` with `SymbolConfig`, `SymbolMap`, `StooqQuotes`.
`StooqQuotes` implements `Quotes` by fetching from stooq's CSV endpoint,
parsing `Close`, applying `divisor`, and returning `Quote { price }`.

### Architecture

`StooqQuotes` is stateless: one `reqwest::Client` (connection-pool is
internally shared) and an immutable `SymbolMap`. The CSV response has a header
row; the data row is at index 1. `Close` is at column index 6
(`Symbol,Date,Time,Open,High,Low,Close,Volume`). Parse with the `csv` crate
(already a dependency). `divisor` defaults to 1 if `None`.

Errors:

- Symbol absent from map → `anyhow::bail!`
- HTTP error → propagate via `?`
- CSV parse failure → propagate via `context`
- `Close == "N/D"` → `anyhow::bail!`

`from_toml_file` reads the file to a `String`, parses with
`toml::from_str::<HashMap<String, SymbolConfig>>(...)`.
`SymbolConfig` derives `serde::Deserialize`.

### File Manifest

| File | Change |
| --- | --- |
| `src/stooq.rs` | **Create** |
| `src/lib.rs` | Add `pub mod stooq;` |

### WBS

1. Create `src/stooq.rs`.
2. Add imports: `async_trait::async_trait`, `reqwest`, `rust_decimal::Decimal`,
   `std::collections::HashMap`, `std::path::Path`, `serde::Deserialize`,
   `crate::transactions::{Quote, Quotes}`.
3. Define `#[derive(Deserialize)] pub struct SymbolConfig { pub suffix: String, pub divisor: Option<u32> }`.
4. Define `pub type SymbolMap = HashMap<String, SymbolConfig>`.
5. Define `pub struct StooqQuotes { client: reqwest::Client, map: SymbolMap }`.
6. Implement `StooqQuotes::new(map: SymbolMap) -> Self` — calls `reqwest::Client::new()`.
7. Implement `StooqQuotes::from_toml_file(path: &Path) -> anyhow::Result<Self>`:
   - `std::fs::read_to_string(path)?`
   - `toml::from_str::<SymbolMap>(&text)?`
   - `Ok(Self::new(map))`
8. Add `#[async_trait] impl Quotes for StooqQuotes`:
   - Look up `self.map.get(symbol)` — error if missing
   - Build URL: `format!("https://stooq.com/q/l/?s={}{}&f=sd2t2ohlcv&h&e=csv", symbol.to_lowercase(), config.suffix)`
   - `let text = self.client.get(&url).send().await?.error_for_status()?.text().await?`
   - Parse with `csv::ReaderBuilder::new().has_headers(true).from_reader(text.as_bytes())`
   - Read single record; get field at index 6 (`Close`)
   - Error if `close == "N/D"`
   - Parse `Decimal` from close string
   - Apply divisor: `if let Some(d) = config.divisor { price /= Decimal::from(d); }`
   - Return `Ok(Quote { price })`
9. In `src/lib.rs`, add `pub mod stooq;`.
10. Add unit tests in `src/stooq.rs` `#[cfg(test)]` block:
    - `price_unknown_symbol_errors`: construct `StooqQuotes::new(HashMap::new())`,
      call `.price("AAPL").await`, assert `is_err()`
    - `price_nd_returns_error`: build a fake CSV response string; need to test
      parsing logic in isolation — extract CSV parsing into a private helper
      `fn parse_close(csv_text: &str) -> anyhow::Result<Decimal>` and test that
    - `price_divisor_applied`: call `parse_close` with valid CSV, supply divisor=100,
      assert result equals expected divided value

### Verification

```bash
cargo test --lib stooq::
```

Three unit tests pass; no clippy warnings.

---

## Feature 4 — Symbol-Map TOML Config

### Target State

`StooqQuotes::from_toml_file(path)` loads the TOML file. The path comes from
`STOOQ_SYMBOL_MAP` env var (read in `ws.rs` main). The TOML format is covered
by `SymbolConfig` deriving `Deserialize`. This feature has no separate file;
it is part of `src/stooq.rs` (Feature 3, step 7) and `src/ws.rs` (Feature 5).

### Architecture

`toml::from_str::<HashMap<String, SymbolConfig>>` maps each `[SYMBOL]` table
section to a `SymbolConfig`. `suffix` is required; `divisor` is `Option<u32>`.
A missing file or invalid TOML causes `from_toml_file` to return `Err`, which
is treated as a fatal startup error in `main()`.

### File Manifest

| File | Change |
| --- | --- |
| `src/stooq.rs` | Covered by Feature 3 WBS step 7 |
| `src/ws.rs` | Covered by Feature 5 WBS |

### WBS

No additional steps beyond Features 3 and 5. TOML parsing is implemented in
`from_toml_file` (Feature 3, step 7) and called from `main()` (Feature 5).

### Verification

Covered by Feature 3 and Feature 5 verification.

---

## Feature 5 — Provider Selection and Web Service Wiring

### Target State

`AppState` gains `quotes: Arc<dyn Quotes>`. `main()` reads
`WADDLE_QUOTES_PROVIDER` (fatal if absent/unrecognised). For `stooq` it also
reads `STOOQ_SYMBOL_MAP` and calls `StooqQuotes::from_toml_file` (fatal on
error). `get_portfolio` calls `portfolio.valuation(&*s.quotes).await` and maps
`Err` to `StatusCode::INTERNAL_SERVER_ERROR`.

### Architecture

`Arc<dyn Quotes>` in `AppState` requires `Quotes: Send + Sync` (already a
supertrait after Feature 1). Axum requires `AppState: Clone`; `Arc<dyn Quotes>`
is `Clone`. `get_portfolio` is now `async` and awaits `valuation()` — it was
already `async fn`, no signature change needed.

The `MutexGuard` over transactions must be dropped before the `.await` on
`valuation()` because `MutexGuard` is not `Send`. Clone the transactions inside
a scoped block before calling `valuation`.

`main()` uses `.unwrap_or_else(|e| { eprintln!(...); std::process::exit(1) })`
or `anyhow::Context` plus `process::exit(1)` for fatal errors.

### File Manifest

| File | Change |
| --- | --- |
| `src/ws.rs` | Update `AppState`, `main`, `get_portfolio`, tests |

### WBS

1. Add import in `ws.rs`: `use vigilant_waddle::stooq::StooqQuotes;`
2. Add `quotes: Arc<dyn Quotes>` field to `AppState`.
3. Update `main()`:
   - Read `WADDLE_QUOTES_PROVIDER`; fatal if not `"stooq"` or `"mock"`
   - For `"stooq"`: read `STOOQ_SYMBOL_MAP`, call `StooqQuotes::from_toml_file(&path)`;
     fatal on error
   - For `"mock"`: create `Arc::new(MockQuotes::new())`
   - Pass `quotes` into `AppState`
4. Update `get_portfolio`:
   - Clone transactions inside a scoped block so `MutexGuard` is dropped before `.await`
   - `let portfolio = Portfolio::from_transactions(&t);`
   - `let val = portfolio.valuation(&*s.quotes).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;`
   - Return `Ok(Json(val))`
5. Update `empty_state()` in tests to include `quotes: Arc::new(MockQuotes::new())`.
6. Update `test_get_portfolio_after_upload`: change `currency` assertion from
   `"USD"` to `"BAR"` (currency now comes from the CSV `currency` field).
7. Remove now-unused `Quotes` import from the `use` statement if only needed
   for the old inline construction.

### Verification

```bash
cargo test --lib
cargo clippy -- -W clippy::pedantic
```

All tests pass; no clippy warnings across the entire crate.

---

## Cross-Cutting: Dependency Order

Implement in this order to minimize cascading compile errors:

1. `Cargo.toml` — add dependencies first
2. Feature 1 (`Quote`, `Quotes`, `MockQuotes` in `transactions.rs`)
3. Feature 2 (`Portfolio`, `PortfolioValuation` in `transactions.rs`)
4. Feature 3 + 4 (`src/stooq.rs` + `lib.rs`)
5. Feature 5 (`ws.rs`)

## Files to Create / Modify

| Action | Path |
| --- | --- |
| **Modify** | `Cargo.toml` |
| **Modify** | `src/transactions.rs` |
| **Create** | `src/stooq.rs` |
| **Modify** | `src/lib.rs` |
| **Modify** | `src/ws.rs` |

## Verification

After all features are implemented:

```bash
cargo test --lib
cargo clippy -- -W clippy::pedantic
markdownlint-cli2 doc/stooq/PLAN.md
```

All three commands must exit with code 0.
