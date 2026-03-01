# Spec: Portfolio Valuation (Eval Cycle)

## Objective

Extend portfolio evaluation to include current market values. Introduce a `Quotes`
abstraction in the `transactions` module so positions can be priced, add a mock
implementation for testing, surface values in the `GET /portfolio` response, and
update the frontend to display the enriched data.

## Tech Stack

- Rust — Axum, serde, rust\_decimal (no new crates)
- Vanilla JS — no new dependencies

## Core Features

1. **`Quotes` trait** — Abstract interface in `src/transactions.rs` for fetching
   the current price of a symbol.
2. **`MockQuotes` implementation** — HashMap-backed struct implementing `Quotes`,
   configurable with arbitrary symbol → (price, currency) pairs.
3. **`Portfolio::from_transactions_with_quotes`** — New constructor that computes
   value per position (`quantity × quote_price`) using a `Quotes` reference.
4. **`GET /portfolio` format change** — Response switches from `application/yaml`
   to `application/json`; body is a JSON array including value and currency fields.
5. **Frontend update** — Table extended with Value and Currency columns; custom
   YAML parser replaced by JSON parsing.

## Public Interface & Data Structures

### `Quotes` trait (new, `src/transactions.rs`)

```rust
pub struct Quote {
    pub price: Decimal,
    pub currency: String,
}

pub trait Quotes {
    /// Returns the current quote for `symbol`, or an error if unavailable.
    fn price(&self, symbol: &str) -> anyhow::Result<Quote>;
}
```

### `MockQuotes` struct (new, `src/transactions.rs`)

`MockQuotes` never returns an error. Any symbol not explicitly configured falls
back to a default fake quote (`price: 1.00, currency: "USD"`), making it always
infallible for tests and as the production stand-in this cycle.

```rust
pub struct MockQuotes {
    data: HashMap<String, Quote>,
}

impl MockQuotes {
    pub fn new() -> Self;
    pub fn insert(
        &mut self,
        symbol: impl Into<String>,
        price: Decimal,
        currency: impl Into<String>,
    );
}

impl Quotes for MockQuotes {
    // Always returns Ok — configured quote if present, default fake quote otherwise.
    fn price(&self, symbol: &str) -> anyhow::Result<Quote>;
}
```

### `Portfolio` — new constructor (extends `src/transactions.rs`)

```rust
impl Portfolio {
    pub fn from_transactions_with_quotes(
        t: &Transactions,
        quotes: Arc<dyn Quotes + Send + Sync>,
    ) -> Self;
}
```

Infallible. Quote errors are deferred to serialization time and surface as
HTTP 500.

### `GET /portfolio` response (JSON array)

```json
[
  { "symbol": "FOO", "quantity": 3, "value": "330.00", "currency": "USD" },
  { "symbol": "BAR", "quantity": -1, "value": "-95.50", "currency": "EUR" }
]
```

Every entry always includes `value` and `currency` (quote errors surface as
HTTP 500).

### `AppState` (`src/ws.rs`)

```rust
struct AppState {
    transactions: Arc<Mutex<Option<Transactions>>>,
}
```

`AppState` is unchanged. `MockQuotes` is instantiated inside `get_portfolio`
on each request; no quotes field is added to the shared state.

## Acceptance Criteria

1. `GET /portfolio` returns `200 OK`, `Content-Type: application/json`, and a JSON
   array where every entry has `symbol`, `quantity`, `value`, and `currency`.
2. `value` is computed as `quantity × quote_price` with `rust_decimal` precision.
3. `GET /portfolio` returns `500 Internal Server Error` if any quote lookup fails.
4. `GET /portfolio` returns `404 Not Found` when no transactions are loaded.
5. Unit test: `MockQuotes::price` returns the configured quote for known symbols
   and the default fake quote (`price: 1.00, currency: "USD"`) for unknown
   symbols — never errors.
6. Unit test: `Portfolio::from_transactions_with_quotes` computes correct values,
   including multi-currency positions and short (negative quantity) positions.
7. Integration test: `GET /portfolio` after upload returns JSON array with correct
   `value`/`currency` fields for all symbols.
8. E2E hurl test (`hurl/portfolio.hurl`) updated to assert the new JSON format.
9. Frontend table columns: Symbol | Quantity | Value | Currency.
10. All previously passing unit and integration tests continue to pass.
11. `cargo clippy -- -W clippy::pedantic` emits no warnings.
12. All Markdown files pass `markdownlint-cli2`.

## Constraints

- No new Cargo dependencies.
- `GET /portfolio` content type changes from `application/yaml` to `application/json`
  (breaking change — frontend updated in the same cycle).
- Currency conversion is out of scope; each position reports its quote's native
  currency.
- A missing quote is always an error; partial portfolios are not supported.
- `MockQuotes` is the production implementation this cycle; the trait design allows
  a real quotes source to replace it in a future cycle.
- Ansible and Docker infrastructure require no changes.
- Code conventions: higher-level functions first; pedantic Clippy required.
