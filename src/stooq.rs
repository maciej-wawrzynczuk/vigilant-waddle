use anyhow::Context as _;
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::transactions::{Quote, Quotes};

#[derive(Deserialize)]
pub struct SymbolConfig {
    pub suffix: String,
    pub divisor: Option<u32>,
}

pub type SymbolMap = HashMap<String, SymbolConfig>;

#[derive(Deserialize)]
struct SymbolMapFile {
    symbols: SymbolMap,
}

pub struct StooqQuotes {
    client: reqwest::Client,
    pub(crate) map: SymbolMap,
}

impl StooqQuotes {
    #[must_use]
    pub fn new(map: SymbolMap) -> Self {
        Self {
            client: reqwest::Client::new(),
            map,
        }
    }

    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid TOML.
    pub fn from_toml_file(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read symbol map from {}", path.display()))?;
        let file = toml::from_str::<SymbolMapFile>(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(Self::new(file.symbols))
    }
}

#[async_trait]
impl Quotes for StooqQuotes {
    async fn price(&self, symbol: &str) -> anyhow::Result<Quote> {
        let config = self
            .map
            .get(symbol)
            .with_context(|| format!("symbol '{symbol}' not found in symbol map"))?;

        let url = format!(
            "https://stooq.com/q/l/?s={}{}&f=sd2t2ohlcv&h&e=csv",
            symbol.to_lowercase(),
            config.suffix
        );

        let text = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let price = parse_close(&text)?;
        let price = if let Some(d) = config.divisor {
            price / Decimal::from(d)
        } else {
            price
        };

        Ok(Quote { price })
    }
}

fn parse_close(csv_text: &str) -> anyhow::Result<Decimal> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_text.as_bytes());

    let record = rdr
        .records()
        .next()
        .context("no data row in stooq CSV response")?
        .context("failed to read stooq CSV record")?;

    // Column layout: Symbol,Date,Time,Open,High,Low,Close,Volume
    let close = record.get(6).context("missing Close column in stooq CSV")?;

    anyhow::ensure!(
        close != "N/D",
        "stooq returned N/D for Close — symbol may be invalid or market closed"
    );

    close
        .parse::<Decimal>()
        .with_context(|| format!("failed to parse Close value '{close}' as Decimal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_temp_toml(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn from_toml_file_missing_file_errors() {
        let path = std::path::PathBuf::from("/nonexistent/path/symbols.toml");
        let err = StooqQuotes::from_toml_file(&path).err().unwrap();
        assert!(
            err.to_string().contains("failed to read symbol map from"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_toml_file_invalid_toml_errors() {
        let path = write_temp_toml("stooq_invalid.toml", "[[[ not valid toml");
        let err = StooqQuotes::from_toml_file(&path).err().unwrap();
        assert!(
            err.to_string().contains("failed to parse"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_toml_file_flat_keys_errors() {
        // Old flat format — missing [symbols] wrapper — must be rejected.
        let toml = "[IBM]\nsuffix = \".US\"\n";
        let path = write_temp_toml("stooq_flat.toml", toml);
        let err = StooqQuotes::from_toml_file(&path).err().unwrap();
        assert!(
            err.to_string().contains("failed to parse"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_toml_file_valid_nested_format_parses() {
        let toml =
            "[symbols.IBM]\nsuffix = \".US\"\n\n[symbols.GSK]\nsuffix = \".UK\"\ndivisor = 100\n";
        let path = write_temp_toml("stooq_valid.toml", toml);
        let sut = StooqQuotes::from_toml_file(&path).unwrap();
        let ibm = sut.map.get("IBM").unwrap();
        assert_eq!(ibm.suffix, ".US");
        assert!(ibm.divisor.is_none());
        let gsk = sut.map.get("GSK").unwrap();
        assert_eq!(gsk.suffix, ".UK");
        assert_eq!(gsk.divisor, Some(100));
    }

    #[tokio::test]
    async fn price_unknown_symbol_errors() {
        let sut = StooqQuotes::new(HashMap::new());
        assert!(sut.price("AAPL").await.is_err());
    }

    #[test]
    fn price_nd_returns_error() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-01-01,12:00:00,100,110,90,N/D,0\n";
        assert!(parse_close(csv).is_err());
    }

    #[test]
    fn price_divisor_applied() {
        let csv = "Symbol,Date,Time,Open,High,Low,Close,Volume\n\
                   AAPL.US,2026-01-01,12:00:00,100,110,90,10000,1000\n";
        let raw = parse_close(csv).unwrap();
        let divisor = 100u32;
        let result = raw / Decimal::from(divisor);
        assert_eq!(result, "100".parse::<Decimal>().unwrap());
    }
}
