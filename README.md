# My Always Unfinished Transaction System

## What I want to do

- Load a transaction log from a csv file - done
- Get quotes from Stooq
- Evaluate how the portfolio performs:
  - Calculate daily return: `(Value(t) - Value(t-1)) / Value(t-1)`. Utilize static
    content, and if changes occur between periods, calculate `t-1` using `t`
    content. If it changed in between, the `t-1` value is calculated using `t`
    content.
  - Draw a chart. Using annualized values and SMA smoothing.

## Techstack

- Rust
- Polars
- Webservice with axum

## Development workflow

- `devel` is the main working branch.
- Pushing to `main` or `dev-build` triggers a GitHub Actions workflow that
  runs tests, builds the binary, and pushes the Docker image
  (`maciekw/waddle-ws`) to Docker Hub. Build status is visible in the
  repository's Actions tab.
- Local deployment uses `docker compose up -d` (serves the UI on port 8080).
- E2E tests can be run against the local deployment (implementation pending).
