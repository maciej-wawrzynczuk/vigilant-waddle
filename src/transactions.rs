use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io::Read;

pub struct Portfolio {
    // I use a vec instead of a hash. The list is short.
    // There will be more scans when lookups.
    data: Vec<(String, i32)>,
}

impl Default for Portfolio {
    fn default() -> Self {
        Self::new()
    }
}

impl Portfolio {
    #[must_use]
    pub fn from_transactions(t: &Transactions) -> Self {
        let mut p = Self::new();
        t.iter().for_each(|tx| p.add_transaction(tx));
        p
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Vec::<(String, i32)>::new(),
        }
    }

    pub fn add_transaction(&mut self, t: &MyTransaction) {
        match self.data.iter_mut().find(|p| p.0 == t.symbol) {
            Some(p) => p.1 += t.number,
            None => self.data.push((t.symbol.clone(), t.number)),
        }
    }
}

impl Serialize for Portfolio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.data.len()))?;
        for (symbol, qty) in &self.data {
            map.serialize_entry(symbol, qty)?;
        }
        map.end()
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
    pub fn try_from_reader<R: Read>(rd: R) -> csv::Result<Self> {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .from_reader(rd);

        // Validate headers
        let headers = rdr.headers()?;
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
            return Err(csv::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid CSV headers",
            )));
        }

        let v = rdr
            .into_deserialize()
            .collect::<csv::Result<Vec<MyTransaction>>>()?;

        Ok(Self { p: v })
    }

    pub fn iter(&self) -> impl Iterator<Item = &MyTransaction> + '_ {
        self.p.iter()
    }
}

#[cfg(test)]
mod test {
    use crate::transactions::{Portfolio, Transactions};
    use indoc::indoc;
    use std::io::Cursor;

    #[test]
    fn test_from_csv() {
        let test_rd = Cursor::new(indoc! {"
            date;symbol;number;price;commission;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
        "});

        let sut = Transactions::try_from_reader(test_rd).unwrap();
        sut.iter().next().unwrap();
    }

    #[test]
    fn portfolio_yaml_single() {
        let t = Transactions::try_from_reader(Cursor::new(indoc! {"
            date;symbol;number;price;commission;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
        "}))
        .unwrap();
        let sut = Portfolio::from_transactions(&t);
        let yaml = serde_yaml::to_string(&sut).unwrap();
        assert!(yaml.contains("FOO: 1"), "yaml was: {yaml}");
    }

    #[test]
    fn portfolio_yaml_multiple_symbols() {
        let t = Transactions::try_from_reader(Cursor::new(indoc! {"
            date;symbol;number;price;commission;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
            2000-01-01;BAZ;1;42.42;4.2;BAR
        "}))
        .unwrap();
        let sut = Portfolio::from_transactions(&t);
        let yaml = serde_yaml::to_string(&sut).unwrap();
        assert!(yaml.contains("FOO: 1"), "yaml was: {yaml}");
        assert!(yaml.contains("BAZ: 1"), "yaml was: {yaml}");
    }

    #[test]
    fn portfolio_yaml_accumulated() {
        let t = Transactions::try_from_reader(Cursor::new(indoc! {"
            date;symbol;number;price;commission;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
            2000-01-02;FOO;1;42.42;4.2;BAR
        "}))
        .unwrap();
        let sut = Portfolio::from_transactions(&t);
        let yaml = serde_yaml::to_string(&sut).unwrap();
        assert!(yaml.contains("FOO: 2"), "yaml was: {yaml}");
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
