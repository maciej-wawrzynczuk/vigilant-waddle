use log;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::Utc;
use url::Url;
use reqwest::Client;
use tokio::fs::{create_dir_all, File};
use tokio::io::{BufWriter, copy};
use crate::hivepart;

pub trait DbConfig {
    fn db_base(&self) -> &str;
}


pub async fn stooq_download<T: DbConfig>(symbol: &str, c: &T) -> Result<()> {
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

    let path = full_path(stooq_path(symbol).as_path(), c);
    log::info!("Saving to {}", path.to_str().with_context(|| "as_str failed for path. Why???")?);
    let dir = path.parent().with_context(|| format!("Something wrong with {:?}", path))?;
    create_dir_all(dir).await?;
    let f = File::create(path).await?;

    let mut wr = BufWriter::new(f);
    copy(&mut bytes.as_ref(), &mut wr).await?;

    Ok(())
}

fn full_path<T: DbConfig>(p: &Path, c: &T) -> PathBuf {
    PathBuf::from(c.db_base())
        .join(p)
}

fn stooq_path(symbol: &str) -> PathBuf {
    let now = Utc::now();
    let f = vec![
        ("symbol".to_string(), symbol.to_string()),
        ("year".to_string(), now.format("%Y").to_string()),
        ("month".to_string(), now.format("%m").to_string()),
        ("day".to_string(), now.format("%d").to_string())
    ];
    let ts = now.format("%Y%m%dT%H%M%SZ").to_string();
    let h_part = hivepart::mk_hive(f);

    PathBuf::new()
        .join("raw")
        .join("stooq")
        .join(h_part)
        .join(format!("{ts}.csv"))

    //format!("{DATA_BASE}/raw/stooq/{symbol}/{year}/{month}/{day}/{ts}.csv")
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
    use std::path::Component;
    use crate::raw::stooq::{stooq_url, stooq_path};

    #[test]
    fn test_daily_url() {
        let ticker = "foo";
        let expected = format!("https://stooq.com/q/d/l/?s={ticker}&i=d");
        assert!(stooq_url(ticker).unwrap().to_string() == expected)
    }

    #[test]
    fn test_path() {
        let p = stooq_path("foo");
        let parts: Vec<_> = p.components().collect();
        let p_str = p.to_str().unwrap();
        println!("Returned {p_str}");
        assert_eq!(parts[0], Component::Normal("raw".as_ref()));
        assert_eq!(parts[1], Component::Normal("stooq".as_ref()));
    }
}
