use anyhow::{Context as _, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;

#[derive(Clone)]
pub struct Quote {
    pub price: Decimal,
}

#[async_trait]
pub trait Quotes: Send + Sync {
    /// Returns the current quote for `symbol`.
    ///
    /// # Errors
    ///
    /// Returns an error if the quote source is unavailable or the symbol is invalid.
    async fn price(&self, symbol: &str) -> anyhow::Result<Quote>;
}

pub struct MockQuotes {
    data: HashMap<String, Decimal>,
}

impl Default for MockQuotes {
    fn default() -> Self {
        Self::new()
    }
}

impl MockQuotes {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, symbol: impl Into<String>, price: Decimal) {
        self.data.insert(symbol.into(), price);
    }
}

#[async_trait]
impl Quotes for MockQuotes {
    async fn price(&self, symbol: &str) -> anyhow::Result<Quote> {
        Ok(Quote {
            price: self.data.get(symbol).copied().unwrap_or(Decimal::ONE),
        })
    }
}

pub struct Portfolio {
    // I use a vec instead of a hash. The list is short.
    // There will be more scans than lookups.
    data: Vec<(String, i32, String)>,
}

#[derive(Serialize)]
pub struct PortfolioEntry {
    pub symbol: String,
    pub quantity: i32,
    pub value: String,
    pub currency: String,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct PortfolioValuation(Vec<PortfolioEntry>);

impl Portfolio {
    #[must_use]
    pub fn from_transactions(t: &Transactions) -> Self {
        let mut data: Vec<(String, i32, String)> = Vec::new();
        for tx in t.iter() {
            match data.iter_mut().find(|e| e.0 == tx.symbol) {
                Some(e) => e.1 += tx.number,
                None => data.push((tx.symbol.clone(), tx.number, tx.currency.clone())),
            }
        }
        Self { data }
    }

    /// # Errors
    ///
    /// Returns an error if any quote lookup fails.
    pub async fn valuation(&self, quotes: &dyn Quotes) -> anyhow::Result<PortfolioValuation> {
        let mut entries = Vec::with_capacity(self.data.len());
        for (symbol, qty, currency) in &self.data {
            let quote = quotes.price(symbol).await?;
            let value = (Decimal::from(*qty) * quote.price).to_string();
            entries.push(PortfolioEntry {
                symbol: symbol.clone(),
                quantity: *qty,
                value,
                currency: currency.clone(),
            });
        }
        Ok(PortfolioValuation(entries))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MyTransaction {
    // date;symbol;number;price;commission;currency
    pub date: NaiveDate,
    pub symbol: String,
    pub number: i32,
    pub price: Decimal,
    pub commission: Decimal,
    pub currency: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(transparent)]
pub struct Transactions {
    p: Vec<MyTransaction>,
}

impl Default for Transactions {
    fn default() -> Self {
        Self::new()
    }
}

impl Transactions {
    #[must_use]
    pub fn new() -> Self {
        Self { p: Vec::new() }
    }

    /// # Errors
    ///
    /// Fails when:
    /// - Unable to find headers
    /// - Has bad headers
    /// - Hit format errors.
    pub fn try_from_reader<R: Read>(rd: R) -> Result<Self> {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .from_reader(rd);

        // Validate headers
        let headers = rdr.headers().context("failed to read CSV headers")?;
        let expected = [
            "date",
            "symbol",
            "number",
            "price",
            "commission",
            "currency",
        ];
        if headers.len() != expected.len()
            || !expected.iter().zip(headers.iter()).all(|(e, h)| e == &h)
        {
            anyhow::bail!(
                "invalid CSV headers: expected {expected:?}, got {:?}",
                headers.iter().collect::<Vec<_>>()
            );
        }

        let v = rdr
            .into_deserialize()
            .collect::<csv::Result<Vec<MyTransaction>>>()
            .context("failed to deserialize CSV rows")?;

        Ok(Self { p: v })
    }

    pub fn iter(&self) -> impl Iterator<Item = &MyTransaction> + '_ {
        self.p.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.p.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.p.is_empty()
    }
}

#[cfg(test)]
mod test {
    use crate::transactions::{MockQuotes, Portfolio, Quotes as _, Transactions};
    use indoc::indoc;
    use rust_decimal::Decimal;
    use std::io::Cursor;

    #[test]
    fn test_from_csv() {
        let test_rd = Cursor::new(indoc! {"
            date;symbol;number;price;commission;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
        "});

        let sut = Transactions::try_from_reader(test_rd).unwrap();
        let tx = sut.iter().next().unwrap();
        assert_eq!(tx.symbol, "FOO");
        assert_eq!(tx.number, 1);
        assert_eq!(tx.price, "42.42".parse::<Decimal>().unwrap());
    }

    fn make_transactions(rows: &[(&str, i32)]) -> Transactions {
        let header = "date;symbol;number;price;commission;currency\n";
        let body: String = rows
            .iter()
            .map(|(sym, n)| format!("2000-01-01;{sym};{n};42.42;4.2;BAR\n"))
            .collect();
        Transactions::try_from_reader(Cursor::new(format!("{header}{body}"))).unwrap()
    }

    #[tokio::test]
    async fn mock_quotes_known_symbol() {
        let mut q = MockQuotes::new();
        q.insert("FOO", "5.00".parse::<Decimal>().unwrap());
        let quote = q.price("FOO").await.unwrap();
        assert_eq!(quote.price, "5.00".parse::<Decimal>().unwrap());
    }

    #[tokio::test]
    async fn mock_quotes_unknown_symbol() {
        let q = MockQuotes::new();
        let quote = q.price("UNKNOWN").await.unwrap();
        assert_eq!(quote.price, Decimal::ONE);
    }

    #[tokio::test]
    async fn portfolio_values_computed() {
        let mut q = MockQuotes::new();
        q.insert("FOO", "100.00".parse::<Decimal>().unwrap());
        let t = make_transactions(&[("FOO", 3)]);
        let p = Portfolio::from_transactions(&t);
        let val = p.valuation(&q).await.unwrap();
        let json: serde_json::Value = serde_json::to_value(val).unwrap();
        assert_eq!(json[0]["value"], "300.00");
        assert_eq!(json[0]["currency"], "BAR");
    }

    #[tokio::test]
    async fn portfolio_multi_currency_short() {
        let mut q = MockQuotes::new();
        q.insert("FOO", "100.00".parse::<Decimal>().unwrap());
        q.insert("BAR_SYM", "50.00".parse::<Decimal>().unwrap());
        let csv = "date;symbol;number;price;commission;currency\n\
                   2000-01-01;FOO;3;1.00;0;USD\n\
                   2000-01-01;BAR_SYM;-1;1.00;0;EUR\n";
        let t = Transactions::try_from_reader(Cursor::new(csv)).unwrap();
        let p = Portfolio::from_transactions(&t);
        let val = p.valuation(&q).await.unwrap();
        let json: serde_json::Value = serde_json::to_value(val).unwrap();
        assert_eq!(json[0]["symbol"], "FOO");
        assert_eq!(json[0]["quantity"], 3);
        assert_eq!(json[0]["value"], "300.00");
        assert_eq!(json[0]["currency"], "USD");
        assert_eq!(json[1]["symbol"], "BAR_SYM");
        assert_eq!(json[1]["quantity"], -1);
        assert_eq!(json[1]["value"], "-50.00");
        assert_eq!(json[1]["currency"], "EUR");
    }

    #[tokio::test]
    async fn portfolio_zero_quantity() {
        let mut q = MockQuotes::new();
        q.insert("FOO", "10.00".parse::<Decimal>().unwrap());
        let csv = "date;symbol;number;price;commission;currency\n\
                   2000-01-01;FOO;1;1.00;0;USD\n\
                   2000-01-02;FOO;-1;1.00;0;USD\n";
        let t = Transactions::try_from_reader(Cursor::new(csv)).unwrap();
        let p = Portfolio::from_transactions(&t);
        let val = p.valuation(&q).await.unwrap();
        let json: serde_json::Value = serde_json::to_value(val).unwrap();
        assert_eq!(json[0]["value"], "0");
        assert_eq!(json[0]["currency"], "USD");
    }

    #[test]
    fn test_invalid_csv_fails() {
        let invalid_csv = Cursor::new("invalid,csv,data\n");
        let result = Transactions::try_from_reader(invalid_csv);
        assert!(result.is_err(), "Expected parsing to fail for invalid CSV");
    }

    #[test]
    fn test_wrong_delimiter_fails() {
        let wrong_delimiter = Cursor::new(indoc! {"
            date,symbol,number,price,commission,currency
            2000-01-01,FOO,1,42.42,4.2,BAR
        "});
        let result = Transactions::try_from_reader(wrong_delimiter);
        assert!(
            result.is_err(),
            "Expected parsing to fail with wrong delimiter"
        );
    }

    #[test]
    fn test_missing_required_fields() {
        let missing_fields = Cursor::new(indoc! {"
            date;symbol
            2000-01-01;FOO
        "});
        let result = Transactions::try_from_reader(missing_fields);
        assert!(
            result.is_err(),
            "Expected parsing to fail when required fields are missing"
        );
    }
}
