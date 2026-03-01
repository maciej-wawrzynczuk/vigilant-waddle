<!-- markdownlint-disable MD013 -->
# SPEC: Stooq Quote Provider

## Objective

Replace the hardcoded `MockQuotes` in `GET /portfolio` with a real market-data
implementation backed by the stooq.com free CSV API. Introduce `src/stooq.rs`
— a stateless, async quote provider — make the `Quotes` trait async, refactor
`Portfolio` to carry currency from transaction data and use an explicit async
valuation step. The active provider is selected at startup via an environment
variable, so `MockQuotes` remains available for integration tests and local
development without network access.

## Tech Stack

**Existing (unchanged):** Rust, Axum 0.8, serde/serde_json, rust_decimal,
tokio (full), anyhow, csv, tracing.

**New dependencies:**

| Crate | Features | Reason |
| --- | --- | --- |
| `reqwest` | `rustls-tls` | Async HTTPS client for stooq CSV endpoint |
| `async-trait` | (default) | `dyn`-compatible async trait via `#[async_trait]` |
| `toml` | (default) | Parse the symbol-map configuration file |

`rustls-tls` is preferred over `native-tls` to keep the binary fully
statically linked (compatible with `debian:bookworm-slim`).

## Core Features

### Feature 1 — Async `Quotes` trait

Annotate the `Quotes` trait with `#[async_trait]` and change `price` to
`async fn`. Remove `currency` from `Quote` — currency is owned by the
transaction layer, not the quote provider. Update `MockQuotes` to match.
The `Send + Sync` bounds move from call sites onto the trait as supertraits.

### Feature 2 — `Portfolio` carries currency; `Portfolio::valuation()`

`Portfolio` gains a currency field per holding (taken from transaction data).
Remove the custom `Serialize` impl from `Portfolio` (incompatible with async
trait). Add `Portfolio::valuation(quotes: &dyn Quotes) -> anyhow::Result<PortfolioValuation>`,
which fetches all quotes and pairs prices with the stored currencies.
`PortfolioValuation` derives `Serialize`; its JSON shape is identical to the
current response.

### Feature 3 — `StooqQuotes` provider (`src/stooq.rs`)

A stateless struct holding a `reqwest::Client` and a `SymbolMap`. Implements
`Quotes`. For each `price()` call it looks up the symbol's config, builds the
stooq URL, fetches the CSV, parses `Close`, applies the optional `divisor`,
and returns a `Quote`. Returns an error for unknown symbols, `N/D` values,
network failures, or malformed CSV.

### Feature 4 — Symbol-map TOML config

A TOML file whose path comes from env var `STOOQ_SYMBOL_MAP` maps each
portfolio symbol to a stooq exchange suffix and an optional price divisor.
Currency is **not** in this file — it comes from the transaction CSV.
The file is loaded once at server startup; absent or unreadable file is a
fatal startup error.

### Feature 5 — Provider selection and web service wiring

The active quote provider is chosen at startup by the env var
`WADDLE_QUOTES_PROVIDER`:

| Value | Provider | Extra env var required |
| --- | --- | --- |
| `stooq` | `StooqQuotes` (live stooq data) | `STOOQ_SYMBOL_MAP` |
| `mock` | `MockQuotes` (1.00 fallback) | none |

Unset or unrecognised values are a fatal startup error. `AppState` gains
a `quotes: Arc<dyn Quotes>` field. `get_portfolio` calls
`portfolio.valuation(quotes).await` and maps errors to 500.

## Public Interface and Data Structures

### `Quote` (updated, `src/transactions.rs`)

```rust
pub struct Quote {
    pub price: Decimal,   // currency field removed
}
```

### `Quotes` trait (updated, `src/transactions.rs`)

```rust
#[async_trait]
pub trait Quotes: Send + Sync {
    async fn price(&self, symbol: &str) -> anyhow::Result<Quote>;
}
```

### `Portfolio` (updated, `src/transactions.rs`)

Holds currency per symbol from transaction data.

```rust
pub struct Portfolio {
    data: Vec<(String, i32, String)>,  // (symbol, quantity, currency)
}

impl Portfolio {
    pub fn from_transactions(t: &Transactions) -> Self;
    pub async fn valuation(&self, quotes: &dyn Quotes) -> anyhow::Result<PortfolioValuation>;
}
```

### `PortfolioValuation` (new, `src/transactions.rs`)

Serializes as a JSON array, matching the existing `GET /portfolio` response
shape:

```json
[
  { "symbol": "AAPL", "quantity": 5,  "value": "921.75", "currency": "USD" },
  { "symbol": "VOD",  "quantity": 100, "value": "85.50",  "currency": "GBP" }
]
```

### `StooqQuotes` (new, `src/stooq.rs`)

```rust
pub struct SymbolConfig {
    pub suffix: String,
    pub divisor: Option<u32>,  // divide raw stooq price by this (default: 1)
}
pub type SymbolMap = HashMap<String, SymbolConfig>;

pub struct StooqQuotes {
    client: reqwest::Client,
    map: SymbolMap,
}

impl StooqQuotes {
    pub fn new(map: SymbolMap) -> Self;
    pub fn from_toml_file(path: &std::path::Path) -> anyhow::Result<Self>;
}

#[async_trait]
impl Quotes for StooqQuotes { ... }
```

Stooq URL pattern:

```text
https://stooq.com/q/l/?s={SYMBOL}{SUFFIX}&f=sd2t2ohlcv&h&e=csv
```

Response columns: `Symbol,Date,Time,Open,High,Low,Close,Volume`.
Price = `Close / divisor` (divisor defaults to 1). Returns error if
`Close == "N/D"`.

### `AppState` (updated, `src/ws.rs`)

```rust
#[derive(Clone)]
struct AppState {
    transactions: Arc<Mutex<Option<Transactions>>>,
    quotes: Arc<dyn Quotes>,
}
```

### Environment Variables

| Variable | Required | Description |
| --- | --- | --- |
| `WADDLE_QUOTES_PROVIDER` | Yes | `stooq` or `mock` |
| `STOOQ_SYMBOL_MAP` | When provider is `stooq` | Path to symbol-map TOML file |
| `WADDLE_LISTEN_ADDR` | No | Listen address (default `127.0.0.1:3000`) |
| `RUST_LOG` | No | Log filter |

### Symbol Map TOML Format

Each section header is the symbol as it appears in the transaction CSV.
`suffix` is appended to the symbol when querying stooq. `divisor` is optional
and divides the raw stooq price; use `divisor = 100` for London Stock Exchange
stocks where stooq returns prices in GBX (pence) but transactions are recorded
in GBP (pounds). Currency is not present — it comes from the transaction CSV.

```toml
[AAPL]
suffix = ".US"

[CDR]
suffix = ".PL"

[VOD]
suffix = ".UK"
divisor = 100    # stooq returns GBX (pence); divide by 100 to get GBP
```

Deserialized into `HashMap<String, SymbolConfig>` via serde + `toml` crate.

## Acceptance Criteria

1. `GET /portfolio` with `WADDLE_QUOTES_PROVIDER=stooq` returns `200 OK` with
   `application/json` and an array where each entry has `symbol`, `quantity`,
   `value` (quantity × adjusted price), and `currency` (from transaction CSV).
2. `value` is computed with `rust_decimal` and formatted as a plain decimal
   string (no exponent notation).
3. For symbols with `divisor = 100`, the returned `value` equals
   `quantity × (stooq_close / 100)`.
4. `GET /portfolio` returns `500` when a symbol is absent from the map, when
   stooq returns `N/D`, or on HTTP failure.
5. `GET /portfolio` returns `404` when no transactions are loaded.
6. `GET /portfolio` with `WADDLE_QUOTES_PROVIDER=mock` returns `200 OK` using
   `MockQuotes` — no network access required.
7. The server exits with a non-zero code and a clear error message if
   `WADDLE_QUOTES_PROVIDER` is unset, unrecognised, or if `STOOQ_SYMBOL_MAP`
   is required but missing/unreadable/malformed.
8. Unit test: `StooqQuotes::price` returns an error for a symbol not in the map.
9. Unit test: `StooqQuotes::price` returns an error when `Close == "N/D"`.
10. Unit test: `StooqQuotes::price` applies `divisor` correctly
    (e.g., raw `Close = "850.00"` with `divisor = 100` → `price = 8.50`).
11. Unit test: `Portfolio::valuation()` with `MockQuotes` produces correct
    `value` strings for positive, negative, and zero quantities, and uses
    currency from transaction data.
12. All existing `MockQuotes` and transaction tests continue to pass (updated
    for async signature and removed `currency` field from `Quote`).
13. `cargo clippy -- -W clippy::pedantic` emits no warnings.
14. `cargo test --lib` — all tests pass.
15. `markdownlint-cli2` — all Markdown in `doc/stooq/` passes.

## Constraints

- `StooqQuotes` is stateless: no caching; every `price()` call makes a live
  HTTP request.
- `reqwest` uses `rustls-tls`; no `native-tls`.
- Symbol map is immutable after startup; no hot-reload.
- Symbols absent from the map are hard errors; no default suffix fallback.
- `MockQuotes` stays in `src/transactions.rs`; it is not removed or deprecated.
- `Quote` no longer carries `currency`; that field is sourced exclusively from
  the transaction CSV.
- All existing tests that serialized `Portfolio` directly must be rewritten to
  call `valuation()` with `MockQuotes`; the JSON shape is unchanged.
- Currency conversion between currencies is out of scope.
- Docker/Ansible changes (mounting the TOML file, setting env vars) are out of
  scope for this cycle but should be noted in `BUILD-ISSUES.md` if needed.
