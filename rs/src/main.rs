mod raw;
mod hivepart;
mod bronze;

use crate::raw::stooq::stooq_download;
use crate::bronze::stooq::list_files;
use anyhow::Result;
use env_logger;

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    stooq_download("ads.de").await?;
    list_files()?;
    Ok(())
}

