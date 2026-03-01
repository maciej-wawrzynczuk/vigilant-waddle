use anyhow::{Context as _, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

#[derive(Clone)]
pub struct Quote {
    pub price: Decimal,
    pub currency: String,
}

pub trait Quotes {
    /// Returns the current quote for `symbol`.
    ///
    /// # Errors
    ///
    /// Returns an error if the quote source is unavailable or the symbol is invalid.
    fn price(&self, symbol: &str) -> anyhow::Result<Quote>;
}

pub struct MockQuotes {
    data: HashMap<String, Quote>,
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

    pub fn insert(
        &mut self,
        symbol: impl Into<String>,
        price: Decimal,
        currency: impl Into<String>,
    ) {
        self.data.insert(
            symbol.into(),
            Quote {
                price,
                currency: currency.into(),
            },
        );
    }
}

impl Quotes for MockQuotes {
    fn price(&self, symbol: &str) -> anyhow::Result<Quote> {
        Ok(self.data.get(symbol).cloned().unwrap_or(Quote {
            price: "1.00".parse().expect("hardcoded literal"),
            currency: "USD".into(),
        }))
    }
}

pub struct Portfolio {
    // I use a vec instead of a hash. The list is short.
    // There will be more scans than lookups.
    data: Vec<(String, i32)>,
    quotes: Arc<dyn Quotes + Send + Sync>,
}

impl Portfolio {
    #[must_use]
    pub fn from_transactions_with_quotes(
        t: &Transactions,
        quotes: Arc<dyn Quotes + Send + Sync>,
    ) -> Self {
        let mut data: Vec<(String, i32)> = Vec::new();
        for tx in t.iter() {
            match data.iter_mut().find(|e| e.0 == tx.symbol) {
                Some(e) => e.1 += tx.number,
                None => data.push((tx.symbol.clone(), tx.number)),
            }
        }
        Self { data, quotes }
    }
}

impl Serialize for Portfolio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::{Error as _, SerializeSeq};
        let mut seq = serializer.serialize_seq(Some(self.data.len()))?;
        for (symbol, qty) in &self.data {
            let quote = self.quotes.price(symbol).map_err(S::Error::custom)?;
            let value = (Decimal::from(*qty) * quote.price).to_string();
            seq.serialize_element(&PortfolioEntryView {
                symbol,
                quantity: *qty,
                value,
                currency: quote.currency,
            })?;
        }
        seq.end()
    }
}

// Private serialization-only view — not part of public interface
#[derive(Serialize)]
struct PortfolioEntryView<'a> {
    symbol: &'a str,
    quantity: i32,
    value: String,
    currency: String,
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
    use crate::transactions::{MockQuotes, Portfolio, Quotes, Transactions};
    use indoc::indoc;
    use rust_decimal::Decimal;
    use std::io::Cursor;
    use std::sync::Arc;

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

    #[test]
    fn mock_quotes_known_symbol() {
        let mut q = MockQuotes::new();
        q.insert("FOO", "5.00".parse::<Decimal>().unwrap(), "EUR");
        let quote = q.price("FOO").unwrap();
        assert_eq!(quote.price, "5.00".parse::<Decimal>().unwrap());
        assert_eq!(quote.currency, "EUR");
    }

    #[test]
    fn mock_quotes_unknown_symbol() {
        let q = MockQuotes::new();
        let quote = q.price("UNKNOWN").unwrap();
        assert_eq!(quote.price, Decimal::ONE);
        assert_eq!(quote.currency, "USD");
    }

    #[test]
    fn portfolio_values_computed() {
        let mut q = MockQuotes::new();
        q.insert("FOO", "100.00".parse::<Decimal>().unwrap(), "USD");
        let t = make_transactions(&[("FOO", 3)]);
        let p = Portfolio::from_transactions_with_quotes(&t, Arc::new(q));
        let json: serde_json::Value = serde_json::to_value(p).unwrap();
        assert_eq!(json[0]["value"], "300.00");
        assert_eq!(json[0]["currency"], "USD");
    }

    #[test]
    fn portfolio_multi_currency_short() {
        let mut q = MockQuotes::new();
        q.insert("FOO", "100.00".parse::<Decimal>().unwrap(), "USD");
        q.insert("BAR", "50.00".parse::<Decimal>().unwrap(), "EUR");
        let t = make_transactions(&[("FOO", 3), ("BAR", -1)]);
        let p = Portfolio::from_transactions_with_quotes(&t, Arc::new(q));
        let json: serde_json::Value = serde_json::to_value(p).unwrap();
        assert_eq!(json[0]["symbol"], "FOO");
        assert_eq!(json[0]["quantity"], 3);
        assert_eq!(json[0]["value"], "300.00");
        assert_eq!(json[0]["currency"], "USD");
        assert_eq!(json[1]["symbol"], "BAR");
        assert_eq!(json[1]["quantity"], -1);
        assert_eq!(json[1]["value"], "-50.00");
        assert_eq!(json[1]["currency"], "EUR");
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
