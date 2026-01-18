use log;
use anyhow::{Context, Result};
use url::Url;
use reqwest::Client;
use tokio::fs::{create_dir_all, File};
use tokio::io::{BufWriter, copy};
use crate::paths::{PathMan, DbConfig};


pub async fn stooq_download<T: DbConfig>(symbol: &str, pm: &PathMan<'_, T>) -> Result<()> {
    let mut path = pm.patch_from_tags(&["raw", "stooq",format!("symbol={symbol}").as_str()]);
    path.push("data.csv");

    let url = stooq_url(symbol)?;
    log::info!("Downloading {url}");
    let client = Client::new();
    let response = client
        .get(url)
        .send()
        .await?;
    let bytes = response
        .bytes()
        .await?;

    log::info!("Saving to {}", path.to_str().with_context(|| "as_str failed for path. Why???")?);
    let dir = path.parent().with_context(|| format!("Something wrong with {:?}", path))?;
    create_dir_all(dir).await?;
    let f = File::create(path).await?;

    let mut wr = BufWriter::new(f);
    copy(&mut bytes.as_ref(), &mut wr).await?;

    Ok(())
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
    use crate::stooq_download::{stooq_url};

    #[test]
    fn test_daily_url() {
        let ticker = "foo";
        let expected = format!("https://stooq.com/q/d/l/?s={ticker}&i=d");
        assert!(stooq_url(ticker).unwrap().to_string() == expected)
    }

}
