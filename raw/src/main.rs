use anyhow::Result;
use chrono::Utc;

const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/data";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, {}", stooq_url("ads.de"));

    Ok(())
}

fn stooq_url(ticker: &str) -> String {
    format!("https://stooq.com/q/d/l/?s={ticker}&i=d")
}

fn stooq_path(symbol: &str) -> String {
    let now = Utc::now();
    let year = now.format("%Y").to_string();
    let month = now.format("%m").to_string();
    let day = now.format("%d").to_string();
    let ts = now.format("%Y%m%dT%H%M%SZ").to_string();
    format!("{DATA_BASE}/raw/stooq/{symbol}/{year}/{month}/{day}/{ts}.csv")
}

#[cfg(test)]
mod test {
    use regex::Regex;

    #[test]
    fn test_daily_url() {
        let ticker = "foo";
        let expected = format!("https://stooq.com/q/d/l/?s={ticker}&i=d");
        assert!(crate::stooq_url(ticker) == expected)
    }

    #[test]
    fn test_path() {
        //  /home/maciekw/proj/vigilant-waddle/data/raw/stooq/foo/2026/01/11/20260111T115935Z.csv
        let r = Regex::new(r"/home/maciekw/proj/vigilant-waddle/data/raw/stooq/foo/\d{4}/\d{2}/\d{2}/.+csv").unwrap();
        let p = crate::stooq_path("foo");
        println!("Returned {p}");
        assert!(r.is_match(p.as_str()));
    }

}
