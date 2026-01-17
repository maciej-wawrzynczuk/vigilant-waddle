mod config;
mod raw;
mod hivepart;
mod bronze;

use crate::raw::stooq::stooq_download;
use crate::bronze::stooq::list_files;
use anyhow::Result;
use env_logger;
use futures::future::join_all;

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    let c = crate::config::SillyConfig;

    let symbols = vec!["ads.de", "ibm.us"];
    env_logger::init();
    let futs = symbols.into_iter().map(|s| stooq_download(s, &c));
    join_all(futs).await;
    list_files()
        .for_each(|f| println!("{}", f.display()));

    Ok(())
}

