# My always unfinished transaction system

## What I want to do

- Load a transaction log from a csv file - done
- Get quotes from Stooq
- Evaluate - how the portfolio performs:
  - Calculate daily return: (Value(t) - Value(t-1)) / Value(t-1).
     Utilize static content, and if changes occur between periods, calculate “t-1” using “t” content.
     If it changed in between, the "t-1" value is calculated usint "t" content.
  - Draw a chart. Using annualized values and SMA smoothing.

## Techstack

- Rust
- Polars
- Webservice with axum
