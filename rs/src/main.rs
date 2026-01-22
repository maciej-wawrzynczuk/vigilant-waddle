mod stooq_download;

use crate::stooq_download::stooq_download;
use anyhow::{Context, Result};
use env_logger;
use log;
use futures::future::join_all;
use std:: path::{Path, PathBuf};
use std::fs::File;
use arrow::csv::ReaderBuilder;
use arrow::datatypes::{Field, Schema, DataType};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli{
    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Subcommand)]
enum Commands {
    /// Downloads quotes from stooq
    Stooq {
        #[arg(short, long)]
        symbol: String
    }
}

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    Cli::parse();

    let the_first_path = download_example()
        .await
        .next().with_context(|| "None downloaded")?;


   mk_arrow(&the_first_path)?
       .iter()
       .for_each(|br| println!("{}", br.num_rows()));

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

fn mk_arrow(p: &Path) -> Result<Vec<RecordBatch>> {

    let f = File::open(p)?;

    let schema = Schema::new(vec![
        Field::new("date", DataType::Date32, false),
        Field::new("open", DataType::Float64, false),
        Field::new("high", DataType::Float64, false),
        Field::new("low", DataType::Float64, false),
        Field::new("close", DataType::Float64, false),
        Field::new("volume", DataType::Int64, false),
    ]); 

    let rd = ReaderBuilder::new(Arc::new(schema))
        .with_header(true)
        .build(&f)?;

    Ok(rd.filter_map(|r| {
        match r {
            Ok(br) => Some(br),
            Err(e) => {
                log::error!("{e}");
                None
            }
        }
    })
    .collect())
}
