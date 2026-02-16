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

### Transaction log - done

- Import from CSV
- Display

- [x] Make Docker build locally
- [x] Do it with ansible
- [x] Learn naming and tagging docker images in ansible
- [x] Add cleanup code to the playbook.
- [x] Review ai transaction to yaml code
- [x] learn server state
- [x] Create get transactions handler
- [x] upload transactions
- [x] create a e2e test to see if transactions works.

### Portfolio

- Replay transaction log a show the current one

- [ ] create 'portfolio' handler

### Quotes

- Import quotes from Stooq?
- Save to file
- Lookup

### Evaluate portfolio

## Ideas

### Smaller image

It uses debian slim and tini now. Consider ideas:

- Add tini to distroless
- Add ctrl-c handler to rust code
- Musl build

## Dependencies

- docker collection
- requests package
