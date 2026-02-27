# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Build, Lint, and Test

```bash
# Build webservice binary
cargo build --bin waddle-ws

# Build release binary
cargo build --release

# Format code
cargo fmt

# Check code
cargo check

# Lint with pedantic clippy (required for all Rust code)
cargo clippy -- -W clippy::pedantic

# Run all unit/integration tests
cargo test --lib

# Run a single test by name
cargo test transactions::test::test_from_csv -- --exact --nocapture

# Run all tests in a module
cargo test transactions::

# Build Docker image and run E2E tests (requires Docker + Ansible + hurl)
ansible-playbook main-playbook.yml
```

## Architecture

This is a Rust financial portfolio transaction system exposed as a REST API.

**`waddle-ws`** (`src/ws.rs`): Axum REST API server. Two endpoints:

- `PUT /transactions` — upload CSV (multipart form data), stored in
  `Arc<Mutex<Option<Transactions>>>`
- `GET /transactions` — retrieve as JSON

Core modules:

- **`src/transactions.rs`**: Domain logic. `MyTransaction` (single row from
  semicolon-delimited CSV), `Transactions` (collection with CSV parsing and
  date iteration), `Portfolio` (symbol → quantity map).
- **`src/lib.rs`**: Exports the `transactions` module for use by the binary.

E2E tests use [hurl](https://hurl.dev/) files in `hurl/` and are orchestrated
by `main-playbook.yml` (builds Docker image, starts container, runs hurl,
cleans up).

Infrastructure: Docker (`Dockerfile`, `docker-compose.yml` with nginx on port
8080), Ansible (`main-playbook.yml`).

## Workflow (Cycles and Stages)

Development follows a structured cycle. Always ask which cycle and stage we are
in — do not advance stages autonomously. Commit messages must include cycle,
stage, and phase.

1. **Specification** — Create `SPEC.md` from a `SCRATCH.md`. Include:
   objective, tech stack, core features, acceptance criteria, constraints,
   data structures.
2. **Plan** — Create `PLAN.md` from `SPEC.md`. Each section maps to a core
   feature and includes target state, architecture, file manifest, WBS, and
   verification steps.
3. **Coding** — Implement from `PLAN.md`. Log problems and fixes in
   `BUILD-ISSUES.md`.
4. **Acceptance** — Validate against `SPEC.md` and `PLAN.md`; output test
   logs.

Cycle documentation lives in subdirectories of `doc/`.

## Code Conventions

- **Higher-level functions first** — helpers go below callers in source files.
- **Stick to existing toolset** — don't introduce new libraries without
  asking; suggest if significantly better.
- **Docker**: prefer `debian:bookworm-slim` base images.
- **Ansible**: single playbook with a tags system to express task dependencies
  (e.g., a task tagged `[build, deploy]` runs for both `build` and `deploy`
  tags). All Ansible code must pass `ansible-lint`.
- **Markdown**: always comply with markdownlint; correct formatting, spelling,
  and grammar proactively. All markdown must pass `markdownlint-cli2`.
