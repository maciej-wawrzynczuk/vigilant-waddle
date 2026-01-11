mod raw;
mod hivepart;

use crate::raw::stooq::stooq_download;
use anyhow::Result;

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    stooq_download("ads.de").await?;
    Ok(())
}

