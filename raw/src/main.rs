use anyhow::Result;
use chrono::Utc;

const DATA_BASE: &str = "/home/maciekw/proj/vigilant-waddle/";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, {}", stooq_url("ads.de"));

    Ok(())
}

fn stooq_url(ticker: &str) -> String {
    format!("https://stooq.com/q/d/l/?s={ticker}&i=d")
}

fn iso8601now() -> String {
    let now_date = Utc::now();
    now_date.format("%Y-%m-%d").to_string()
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
    fn test_now() {
        let r = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
        let now = crate::iso8601now();
        println!("func returned {})", now);
        assert!(r.is_match(now.as_str()));


    }

}
