mod dataframe;
mod stooq_download;
mod transactions;

use std::path::PathBuf;

use crate::{dataframe::load_csv, stooq_download::stooq_download};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about,
    long_about = None,
    subcommand_required = true,
    arg_required_else_help = true
    )]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Downloads quotes from stooq
    Stooq {
        #[arg(short, long)]
        symbol: String,
    },
    Dataframe {
        #[arg(short, long)]
        symbol: String,
    },
    Transactions {
        #[arg(short, long)]
        file: PathBuf,
    },
}

pub const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Stooq { symbol }) => {
            stooq_download(symbol).await?;
        }
        Some(Commands::Dataframe { symbol }) => {
            let df = load_csv(symbol)?;
            println!("{df}");
        }
        Some(Commands::Transactions { file }) => {
            crate::transactions::list_trans(file)?;
        }

        None => {}
    }

    Ok(())
}
