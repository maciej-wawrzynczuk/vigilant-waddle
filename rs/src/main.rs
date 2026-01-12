mod raw;
mod hivepart;

use crate::raw::stooq::stooq_download;
use anyhow::Result;
use env_logger;

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    stooq_download("ads.de").await?;
    Ok(())
}

