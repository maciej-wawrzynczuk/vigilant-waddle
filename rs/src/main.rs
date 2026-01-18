mod bronze;
mod stooq_download;

use crate::stooq_download::stooq_download;
use crate::bronze::stooq::list_files;
use anyhow::Result;
use env_logger;
use futures::future::join_all;
use std::path::PathBuf;

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

    result.into_iter()
        .filter_map(|r| r.ok())
        .for_each(|i| println!("{}", i.display()));

    Ok(())
}

