mod stooq_download;

use crate::stooq_download::stooq_download;
use anyhow::{Result, Context};
use env_logger;
use futures::future::join_all;
use std::{io::Seek, path::PathBuf};
use std::fs::File;
use arrow::csv::{ReaderBuilder, reader::Format};
use std::sync::Arc;

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

    let mut f = File::open(&the_first_path)?;

    let (schema, _) = Format::default().infer_schema(&f, Some(100))?;
    f.rewind()?;

    let mut rd = ReaderBuilder::new(Arc::new(schema))
        .with_header(true)
        .build(&f)?;

    while let Some(b) = rd.next() {
        let b = b?;
        println!("{b:?}");
    }

    println!("{}", the_first_path.display());

    Ok(())
}
