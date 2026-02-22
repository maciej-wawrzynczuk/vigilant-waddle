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

## The plan

- [x] Make Docker build locally
- [x] Do it with ansible
- [ ] Learn naming and tagging docker images in ansible
- [ ] Add cleanup code to the playbook.

## Ideas
### Smaller image

It uses debian slim and tini now. Consider ideas:
- Add tini to distroless
- Add ctrl-c handler to rust code
- Musl build

## Dependencies

- docker collection
- requests package
