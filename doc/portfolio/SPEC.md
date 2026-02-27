# SPEC: Portfolio Endpoint

## Objective

Add a `GET /portfolio` endpoint to the `waddle-ws` service that computes the
current portfolio from the in-memory transaction log and returns it as YAML.

## Tech Stack

- Rust (existing `vigilant-waddle` crate)
- Axum (existing web framework)
- `serde_yaml` — new dependency for YAML serialization
- Existing `Transactions` and `Portfolio` domain types

## Core Features

1. **Portfolio derivation** — fold all loaded transactions into a `Portfolio`
   (symbol → net quantity), reusing the existing `Portfolio::add_transaction`
   logic.
2. **YAML serialization** — serialize the portfolio as a YAML mapping of
   symbol strings to integer quantities.
3. **`GET /portfolio` endpoint** — return the YAML document; respond `404 Not
   Found` when no transactions have been uploaded yet.

## Data Structures

### YAML response body

A flat mapping from ticker symbol (string) to net share quantity (integer):

```yaml
FOO: 3
BAR: -1
BAZ: 10
```

Symbols with a net quantity of zero are **included** (they represent a
closed position that was once held).

### `Portfolio` (existing, extended)

`Portfolio` already holds `Vec<(String, i32)>`. To support YAML serialization
it must derive `serde::Serialize` and map to the structure above.

## Acceptance Criteria

1. `GET /portfolio` returns `200 OK` with `Content-Type: application/yaml` and
   a well-formed YAML body when transactions are loaded.
2. The YAML body is a mapping of every symbol in the transaction log to its net
   quantity (sum of all `number` values for that symbol).
3. `GET /portfolio` returns `404 Not Found` when no transactions have been
   uploaded.
4. All existing tests continue to pass.
5. A new unit test in `transactions.rs` verifies `Portfolio` YAML
   serialization.
6. A new integration test in `ws.rs` covers the happy path and the 404 case.

## Constraints

- Do not introduce any new library beyond `serde_yaml`.
- The endpoint is read-only; it derives the portfolio at request time from the
  stored `Transactions` — no separate portfolio state is persisted.
- Response content type must be `application/yaml`.
- Follow existing code conventions: higher-level functions above helpers,
  pedantic Clippy must pass, `cargo fmt` must be clean.
