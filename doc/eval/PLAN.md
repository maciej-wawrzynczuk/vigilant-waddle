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

`Portfolio` stores `Vec<(String, i32)>` (symbol + quantity) and an
`Arc<dyn Quotes + Send + Sync>`. Values are computed lazily inside a custom
`Serialize` impl. `from_transactions_with_quotes` is the sole public constructor.

### Architecture

- `Portfolio { data: Vec<(String, i32)>, quotes: Arc<dyn Quotes + Send + Sync> }`.
- `from_transactions_with_quotes(t: &Transactions,
  quotes: Arc<dyn Quotes + Send + Sync>) -> Self`:
  aggregates symbol → quantity inline; infallible.
- Custom `Serialize` impl: iterates `data`, calls `quotes.price(symbol)` per
  entry, computes `value = (Decimal::from(qty) * quote.price).to_string()`,
  serialises via private `PortfolioEntryView<'_>`.
- Private `PortfolioEntryView<'a>` derives `Serialize`; not part of public API.
- Removed: `Portfolio::new()`, `Default`, `add_transaction()`, `from_transactions()`,
  old custom `Serialize` impl.
- Existing YAML portfolio unit tests replaced with two JSON tests:
  `portfolio_values_computed` and `portfolio_multi_currency_short`.

### File Manifest

| File | Change |
|------|--------|
| `src/transactions.rs` | Restructure `Portfolio`; add `from_transactions_with_quotes`; custom `Serialize`; replace YAML tests |

### Work Breakdown

1. Add `use std::sync::Arc` to imports (already present via `std::sync::Arc` in
   `ws.rs`; add to `transactions.rs`).
2. Redefine `Portfolio { data: Vec<(String, i32)>,
   quotes: Arc<dyn Quotes + Send + Sync> }`;
   remove old fields and impls.
3. Implement `Portfolio::from_transactions_with_quotes(t: &Transactions,
   quotes: Arc<dyn Quotes + Send + Sync>) -> Self`:
   - Aggregate quantities inline; return `Self { data, quotes }`.
4. Implement custom `Serialize for Portfolio` using `serialize_seq`; compute
   value per entry at serialization time; delegate to `PortfolioEntryView`.
5. Define private `PortfolioEntryView<'a>` with `#[derive(Serialize)]`.
6. Remove `Portfolio::new()`, `Default`, `add_transaction()`, `from_transactions()`,
   old `Serialize` impl.
7. Replace three YAML tests with:
   - `portfolio_values_computed`: `MockQuotes` with a configured symbol;
     assert `value` and `currency` fields in serialised JSON.
   - `portfolio_multi_currency_short`: negative quantity and two currencies;
     assert both entries correct.

### Verification

`cargo test transactions::` — all tests pass.

---

## Feature 3: `GET /portfolio` JSON Format

### Target State

`get_portfolio` returns `application/json`. `AppState` is **unchanged** — no
`quotes` field added. Handler signature:
`async fn get_portfolio(...) -> Result<Json<Portfolio>, StatusCode>`.

### Architecture

- `AppState` is unchanged; `#[derive(Clone)]` continues to work as-is.
- `main()` requires no changes.
- `get_portfolio`: lock `transactions`, 404 if `None`, construct
  `Arc::new(MockQuotes::new())` locally, call
  `Portfolio::from_transactions_with_quotes(t, quotes)`, return `Json(portfolio)`.
- Quote errors surface as 500 via Axum's JSON serialization error path (no
  explicit `map_err` needed in the handler).
- `Axum`'s `Json<T>` sets `Content-Type: application/json` automatically.
- `serde_yaml` is no longer used; remove its `use` statement from `ws.rs`
  and the dependency from `Cargo.toml`.
- `ws.rs` import: add `MockQuotes`, `Quotes` to `vigilant_waddle::transactions`.

### File Manifest

| File | Change |
|------|--------|
| `src/ws.rs` | Rewrite `get_portfolio`; update imports; update tests |
| `Cargo.toml` | Remove `serde_yaml` (no longer used) |

### Work Breakdown

1. Update `use vigilant_waddle::transactions` to include `MockQuotes`, `Quotes`.
2. Rewrite `get_portfolio` to construct `MockQuotes` locally and return
   `Result<Json<Portfolio>, StatusCode>`.
3. Remove `use serde_yaml` from `ws.rs` (no `serde_yaml` import remains).
4. Remove `serde_yaml` from `Cargo.toml`.
5. `empty_state()` test helper requires **no change** (no quotes field).
6. Update `test_get_portfolio_after_upload`: assert `application/json`
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
