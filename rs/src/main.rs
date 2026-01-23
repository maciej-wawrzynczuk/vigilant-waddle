// TODO: use MIC:Ticker.
// Write mapping to Stooq symbols.
mod stooq_download;
mod dataframe;

use crate::{
    stooq_download::stooq_download,
    dataframe::load_csv
};

use anyhow::Result;
use env_logger;
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
    },
    Dataframe {
        #[arg(short, long)]
        symbol: String
    }
}

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Stooq { symbol}) => {
            stooq_download(symbol).await?;
        },
        Some(Commands::Dataframe { symbol }) => {
            let df = load_csv(symbol)?;
            println!("{df}");
        },
        None => {}
    }

    Ok(())
}

