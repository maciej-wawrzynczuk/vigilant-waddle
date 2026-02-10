use anyhow::{Context, Result};
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs::{File, create_dir_all};
use tokio::io::{BufWriter, copy};
use url::Url;

pub async fn stooq_download(symbol: &str) -> Result<PathBuf> {
    let path = raw_filename(symbol);
    let url = stooq_url(symbol)?;
    log::info!("Downloading {url}");
    let client = Client::new();
    let response = client.get(url).send().await?;
    let bytes = response.bytes().await?;

    log::info!(
        "Saving to {}",
        path.to_str()
            .with_context(|| "as_str failed for path. Why???")?
    );
    let dir = path
        .parent()
        .with_context(|| format!("Something wrong with {:?}", path))?;
    create_dir_all(dir).await?;
    let f = File::create(&path).await?;

    let mut wr = BufWriter::new(f);
    copy(&mut bytes.as_ref(), &mut wr).await?;

    Ok(path)
}

pub fn raw_filename(symbol: &str) -> PathBuf {
    PathBuf::from(crate::DATA_BASE)
        .join("raw")
        .join("stooq")
        .join(format!("symbol={symbol}"))
        .join("data.csv")
}

fn stooq_url(ticker: &str) -> Result<Url, url::ParseError> {
    let mut u = Url::parse("https://stooq.com/q/d/l/")?;
    {
        let mut qp = u.query_pairs_mut();
        qp.append_pair("s", ticker);
        qp.append_pair("i", "d");
    }

    Ok(u)
    // Url::newformat!("https://stooq.com/q/d/l/?s={ticker}&i=d")
}

#[cfg(test)]
mod test {
    use crate::stooq_download::stooq_url;

    #[test]
    fn test_daily_url() {
        let ticker = "foo";
        let expected = format!("https://stooq.com/q/d/l/?s={ticker}&i=d");
        assert!(stooq_url(ticker).unwrap().to_string() == expected)
    }
}
