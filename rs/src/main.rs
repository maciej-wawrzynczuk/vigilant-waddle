mod stooq_download;

use crate::stooq_download::stooq_download;
use anyhow::{Context, Result};
use env_logger;
use log;
use futures::future::join_all;
use std:: path::PathBuf;
use std::fs::File;
use arrow::csv::ReaderBuilder;
use arrow::datatypes::{Field, Schema, DataType};
use std::sync::Arc;

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let the_first_path = download_example()
        .await
        .next().with_context(|| "None downloaded")?;

    let f = File::open(&the_first_path)?;

    let schema = Schema::new(vec![
        Field::new("date", DataType::Date32, false),
        Field::new("open", DataType::Float64, false),
        Field::new("high", DataType::Float64, false),
        Field::new("low", DataType::Float64, false),
        Field::new("close", DataType::Float64, false),
        Field::new("volume", DataType::Int64, false),
    ]); 

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


async fn download_example() -> impl Iterator<Item = PathBuf> {
    let base_path = PathBuf::from(DATA_BASE)
        .join("raw")
        .join("stooq");

    let symbols = vec!["ads.de", "ibm.us"];
    let futs = symbols.into_iter().map(|s| stooq_download(s, &base_path));
    let result = join_all(futs).await;

    result.into_iter()
        .filter_map(|x| {
            match x {
                Ok(v) => Some(v),
                Err(e) => {
                    log::error!("{e}");
                    None
                }
            }
        })
}
