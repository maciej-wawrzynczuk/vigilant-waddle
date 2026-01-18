mod stooq_download;

use crate::stooq_download::stooq_download;
use anyhow::{Result, Context};
use env_logger;
use futures::future::join_all;
use std::path::PathBuf;
use polars::prelude::*;

// use polars_core::prelude::*;
// use polars_io::prelude::*;

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let base_path = PathBuf::from(DATA_BASE)
        .join("raw")
        .join("stooq");

    let symbols = vec!["ads.de", "ibm.us"];
    let futs = symbols.into_iter().map(|s| stooq_download(s, &base_path));
    let result = join_all(futs).await;

    let the_first_path = result.into_iter()
        .filter_map(|x| x.ok())
        .next()
        .context("None downloaded")?;

    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(the_first_path))?
        .finish()?;

    let sh = df.schema();
    let h = df.height();
    println!("{sh:?}");
    println!("{h}");

    Ok(())
}
