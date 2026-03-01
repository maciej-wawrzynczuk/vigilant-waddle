# Plan: Portfolio Valuation (Eval Cycle)

## Overview

Five features implemented bottom-up: library types first (`src/transactions.rs`),
then server (`src/ws.rs`), then E2E (`hurl/`), then frontend (`frontend/`).

---

## Feature 1: `Quote`, `Quotes` Trait, and `MockQuotes`

### Target State

`src/transactions.rs` exports three new public items: `Quote` (value struct),
`Quotes` (trait), and `MockQuotes` (HashMap-backed implementation).

### Architecture

- `Quote { price: Decimal, currency: String }` — derives `Clone`.
- `Quotes` trait: single method `fn price(&self, symbol: &str) -> anyhow::Result<Quote>`.
  Object-safe (no generic methods, no `Self` in return position).
- `MockQuotes { data: HashMap<String, Quote> }` — implements `Default`
  (required by Clippy for structs with a no-arg `new()`).
- Fallback for unknown symbols:
  `Quote { price: Decimal::ONE, currency: "USD".into() }`.
- `MockQuotes` is `Send + Sync` automatically (HashMap<String, Quote> with no
  interior mutability; `Decimal` and `String` are both `Send + Sync`).

### File Manifest

| File | Change |
|------|--------|
| `src/transactions.rs` | Add `use std::collections::HashMap`; add `Quote`, `Quotes`, `MockQuotes` |

### Work Breakdown

1. Add `std::collections::HashMap` to imports.
2. Define `pub struct Quote { pub price: Decimal, pub currency: String }`
   with `#[derive(Clone)]`.
3. Define `pub trait Quotes` with `fn price(&self, symbol: &str) -> anyhow::Result<Quote>`.
4. Define `pub struct MockQuotes { data: HashMap<String, Quote> }`.
5. Implement `Default for MockQuotes` (delegates to `new()`).
6. Implement `MockQuotes::new()` and `MockQuotes::insert(symbol: impl
   Into<String>, price: Decimal, currency: impl Into<String>)`.
7. Implement `Quotes for MockQuotes`: return configured quote if present,
   else default fake quote; always `Ok`.
8. Unit test — known symbol returns configured quote.
9. Unit test — unknown symbol returns `(price: 1.00, currency: "USD")`,
   never errors.

### Verification

`cargo test transactions::` — all tests pass.

---

## Feature 2: `Portfolio` Restructure and `from_transactions_with_quotes`

### Target State

`Portfolio` holds `Vec<PortfolioEntry>` (symbol, quantity, value, currency per
row). `from_transactions_with_quotes` is the sole public constructor.

### Architecture

- `pub struct PortfolioEntry { pub symbol: String, pub quantity: i32,`
  `pub value: String, pub currency: String }` — derives `Serialize`, `Clone`.
  `value` is pre-formatted: `(Decimal::from(qty) * quote.price).to_string()`.
- `Portfolio { entries: Vec<PortfolioEntry> }` — `#[derive(Serialize)]` with
  `#[serde(transparent)]` serialises as a JSON array.
- `from_transactions_with_quotes`: iterates `&Transactions` inline to aggregate
  symbol → quantity (`Vec<(String, i32)>`), then for each entry calls
  `quotes.price(&symbol)?`, computes value string, builds `PortfolioEntry`.
- Removed: `Portfolio::new()`, `Default`, `add_transaction()`, old custom
  `Serialize` impl, `from_transactions()`.
- Existing YAML portfolio unit tests replaced with two JSON tests:
  `portfolio_values_computed` and `portfolio_multi_currency_short`.

### File Manifest

| File | Change |
|------|--------|
| `src/transactions.rs` | Add `PortfolioEntry`; restructure `Portfolio`; add `from_transactions_with_quotes`; replace YAML tests |

### Work Breakdown

1. Define `pub struct PortfolioEntry` with `#[derive(Serialize, Clone)]`.
2. Redefine `Portfolio { entries: Vec<PortfolioEntry> }` with
   `#[derive(Serialize)]` and `#[serde(transparent)]`; remove old fields and impls.
3. Implement `Portfolio::from_transactions_with_quotes(t: &Transactions,
   quotes: &dyn Quotes) -> anyhow::Result<Self>`:
   - Aggregate quantities: iterate `t`, build `Vec<(String, i32)>`.
   - Enrich: for each `(symbol, qty)` call `quotes.price(&symbol)?`,
     compute `value = (Decimal::from(qty) * quote.price).to_string()`.
   - Return `Ok(Self { entries })`.
4. Remove `Portfolio::new()`, `Default`, `add_transaction()`, `from_transactions()`,
   old `Serialize` impl.
5. Replace three YAML tests with:
   - `portfolio_values_computed`: `MockQuotes` with a configured symbol;
     assert `value` and `currency` on the entry.
   - `portfolio_multi_currency_short`: negative quantity and two currencies;
     assert both entries are correct.

### Verification

`cargo test transactions::` — all tests pass.

---

## Feature 3: `GET /portfolio` JSON Format and `AppState` Extension

### Target State

`AppState` carries `quotes: Arc<dyn Quotes + Send + Sync>`. `get_portfolio`
returns `application/json`; handler signature is
`async fn get_portfolio(...) -> Result<Json<Portfolio>, StatusCode>`.

### Architecture

- `AppState` struct gains `quotes: Arc<dyn Quotes + Send + Sync>`.
  `Arc<T>: Clone` for any `T: ?Sized` — `#[derive(Clone)]` continues to work.
- `main()` initialises `quotes: Arc::new(MockQuotes::new())`.
- `get_portfolio`: lock `transactions`, 404 if `None`, call
  `Portfolio::from_transactions_with_quotes(t, &*s.quotes as &dyn Quotes)`,
  map `Err` → 500 (log with `error!`), map `Ok` → `Json(portfolio)`.
- `Axum`'s `Json<T>` extractor sets `Content-Type: application/json` automatically.
- `serde_yaml` is no longer used; remove its `use` statement from `ws.rs`.
  `serde_yaml` may also be removed from `Cargo.toml` (no remaining usage).
- `ws.rs` import: add `MockQuotes`, `Quotes` to `vigilant_waddle::transactions`.

### File Manifest

| File | Change |
|------|--------|
| `src/ws.rs` | Extend `AppState`; rewrite `get_portfolio`; update `main()`; update tests |
| `Cargo.toml` | Remove `serde_yaml` (no longer used) |

### Work Breakdown

1. Update `use vigilant_waddle::transactions` to include `MockQuotes`, `Quotes`.
2. Add `quotes: Arc<dyn Quotes + Send + Sync>` field to `AppState`.
3. Rewrite `get_portfolio` to use `from_transactions_with_quotes` and return
   `Result<Json<Portfolio>, StatusCode>`.
4. Update `main()` to include `quotes: Arc::new(MockQuotes::new())`.
5. Remove `use serde_yaml` from `ws.rs`.
6. Remove `serde_yaml` from `Cargo.toml`.
7. Update `empty_state()` test helper to include
   `quotes: Arc::new(MockQuotes::new())`.
8. Update `test_get_portfolio_after_upload`: assert `application/json`
   content-type; parse body as `serde_json::Value`; assert `symbol`,
   `quantity`, `value`, `currency` fields on the first entry.

### Verification

`cargo test` — all tests pass.
`cargo clippy -- -W clippy::pedantic` — no warnings.

---

## Feature 4: E2E Hurl Test Update

### Target State

`hurl/portfolio.hurl` asserts JSON format with jsonpath assertions on all
four fields.

### Architecture

- `1trans.csv`: symbol `FOO`, quantity `1`. `MockQuotes` default gives
  `price: 1.00, currency: "USD"`, so `value == "1.00"`.
- Replace YAML header and body assertions with JSON content-type and
  jsonpath assertions.

### File Manifest

| File | Change |
|------|--------|
| `hurl/portfolio.hurl` | Replace YAML assertions with JSON jsonpath assertions |

### Work Breakdown

1. Replace `header "Content-Type" contains "application/yaml"` with
   `header "Content-Type" contains "application/json"`.
2. Replace `body contains "FOO: 1"` with:
   - `jsonpath "$[0].symbol" == "FOO"`
   - `jsonpath "$[0].quantity" == 1`
   - `jsonpath "$[0].value" == "1.00"`
   - `jsonpath "$[0].currency" == "USD"`

### Verification

`ansible-playbook main-playbook.yml` — E2E hurl tests pass.

---

## Feature 5: Frontend Update

### Target State

Portfolio table has four columns: Symbol, Quantity, Value, Currency.
`parseYaml` removed; `loadPortfolio` uses `response.json()`; `renderPortfolio`
iterates a JSON array.

### Architecture

- `loadPortfolio`: replace `parseYaml(await res.text())` with `await res.json()`
  (returns array directly).
- `renderPortfolio(data)`: `data` is an `Array`; iterate with
  `for (const entry of data)`, render four `<td>` cells per row.
- Empty-state placeholder: update `colspan` from `2` to `4`.
- `parseYaml` function removed entirely.

### File Manifest

| File | Change |
|------|--------|
| `frontend/index.html` | Add Value/Currency headers; remove `parseYaml`; update render and load functions |

### Work Breakdown

1. Add `<th>Value</th>` and `<th>Currency</th>` to `<thead>`.
2. Update placeholder `<td colspan="2">` to `colspan="4"`.
3. Delete `parseYaml` function.
4. Rewrite `renderPortfolio(data)`:
   - Empty check: `data.length === 0`.
   - Iterate `for (const entry of data)`: render `entry.symbol`,
     `entry.quantity`, `entry.value`, `entry.currency` into four `<td>` cells.
5. In `loadPortfolio`: replace `parseYaml(await res.text())` with
   `await res.json()`.

### Verification

Manual browser test: upload CSV, verify four-column table with correct values.

---

## Global Verification

1. `cargo test --lib` — all tests green.
2. `cargo clippy -- -W clippy::pedantic` — no warnings.
3. `markdownlint-cli2 doc/eval/PLAN.md` — no errors.
4. `ansible-playbook main-playbook.yml` — E2E hurl tests pass.
