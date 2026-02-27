<!-- markdownlint-disable MD024 -->
# PLAN: Portfolio Endpoint

Reference spec: `doc/portfolio/SPEC.md`

---

## Feature 1: YAML serialization of `Portfolio`

### Target state

`Portfolio` serializes to a YAML mapping of symbol → net quantity:

```yaml
FOO: 3
BAR: -1
```

### Architecture

`Portfolio` holds `data: Vec<(String, i32)>`. A plain `#[derive(Serialize)]`
would produce a YAML list of two-element arrays, not a map. Instead,
implement `serde::Serialize` manually using `SerializeMap` so each tuple
becomes a key-value pair.

Add a `Portfolio::from_transactions` constructor (above existing helpers)
that folds a `&Transactions` into a `Portfolio` in one call, keeping
handler code tidy.

### File manifest

| File                    | Change                               |
|-------------------------|--------------------------------------|
| `Cargo.toml`            | Add `serde_yaml = "0.9"`             |
| `src/transactions.rs`   | `Serialize` impl; constructor        |

### WBS

1. Add `serde_yaml = "0.9"` to `Cargo.toml`.
2. Add `use serde::ser::SerializeMap;` import in `transactions.rs`.
3. Implement `Serialize for Portfolio`: iterate `self.data`, emit each
   pair as a map entry.
4. Add `Portfolio::from_transactions(t: &Transactions) -> Portfolio`
   above `Portfolio::new`.

### Verification

- `cargo check` compiles without errors.
- Unit test `portfolio_yaml` (added in Feature 2) passes.

---

## Feature 2: Unit tests for `Portfolio` serialization

### Target state

New tests in `src/transactions.rs` `mod test` confirm correct YAML output.

### Architecture

Use `serde_yaml::to_string(&portfolio)` and assert the result contains
the expected key-value lines. Two cases:

- Single symbol, quantity 1.
- Same symbol in two transactions, accumulated to quantity 2.

### File manifest

| File                    | Change                               |
|-------------------------|--------------------------------------|
| `src/transactions.rs`   | Two new YAML serialization tests     |

### WBS

1. Write `portfolio_yaml_single`: one-entry portfolio, assert
   `yaml.contains("FOO: 1")`.
2. Write `portfolio_yaml_accumulated`: two FOO transactions, assert
   `yaml.contains("FOO: 2")`.

### Verification

- `cargo test transactions::` — both new tests pass alongside existing
  ones.

---

## Feature 3: `GET /portfolio` endpoint

### Target state

`GET /portfolio` returns `200 OK`, `Content-Type: application/yaml`,
and the YAML-serialized portfolio when transactions are loaded;
`404 Not Found` otherwise.

### Architecture

New async handler `get_portfolio` in `src/ws.rs`:

1. Lock `AppState.transactions`.
2. Return `StatusCode::NOT_FOUND` if `None`.
3. Call `Portfolio::from_transactions(t)`.
4. Serialize with `serde_yaml::to_string(&portfolio)` — propagate
   internal error as `500`.
5. Return `([(CONTENT_TYPE, "application/yaml")], body)`.

Register with `.route("/portfolio", axum::routing::get(get_portfolio))`
in `create_app`, alongside the existing `/transactions` routes.

### File manifest

| File          | Change                               |
|---------------|--------------------------------------|
| `src/ws.rs`   | `get_portfolio` handler; new route   |

### WBS

1. Add `Portfolio` to `use vigilant_waddle::transactions` import.
2. Add `response::{IntoResponse, Response}` to `use axum` import.
3. Implement `get_portfolio` handler (described above).
4. Register `GET /portfolio` in `create_app` before `.layer(...)`.

### Verification

- `cargo check` compiles.
- Integration tests (Feature 4) pass.

---

## Feature 4: Integration and E2E tests

### Target state

`ws.rs` test module covers the new endpoint; a hurl file covers E2E.

### Architecture

Two new `#[tokio::test]` functions in `src/ws.rs`:

- `test_get_portfolio_empty` — no transactions uploaded → `404`.
- `test_get_portfolio_after_upload` — upload one FOO transaction, then
  `GET /portfolio` → `200`, body contains `FOO: 1`.

New hurl file `hurl/portfolio.hurl`:

```hurl
PUT http://localhost:3000/transactions
[MultipartFormData]
transaction_log: file,1trans.csv; text/csv

HTTP 200


GET http://localhost:3000/portfolio
HTTP 200
[Asserts]
header "Content-Type" contains "application/yaml"
body contains "FOO: 1"
```

### File manifest

| File                    | Change                               |
|-------------------------|--------------------------------------|
| `src/ws.rs`             | Two new integration tests            |
| `hurl/portfolio.hurl`   | New E2E file                         |

### WBS

1. Write `test_get_portfolio_empty` in `ws.rs`.
2. Write `test_get_portfolio_after_upload` in `ws.rs`.
3. Create `hurl/portfolio.hurl`.

### Verification

- `cargo test --lib` — all tests (old + new) pass.
- `cargo clippy -- -W clippy::pedantic` — no warnings.
- `cargo fmt --check` — clean.
- E2E: `ansible-playbook main-playbook.yml` completes without failures.
