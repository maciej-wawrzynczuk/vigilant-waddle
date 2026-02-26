use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
/// # Errors
///
/// Propagates Err from `Transactions::try_from_reader`
pub fn list_trans(p: &Path) -> anyhow::Result<()> {
    let f = File::open(p)?;
    let rd = BufReader::new(f);
    let t = Transactions::try_from_reader(rd)?;

    t.iter().for_each(|t| println!("{t}"));

    let mut p = Portfolio::new();
    t.iter().for_each(|t| p.add_transaction(t));
    println!("{p}");

    Ok(())
}

#[allow(dead_code)]
pub struct Portfolio {
    data: Vec<(String, i32)>,
}

impl Default for Portfolio {
    fn default() -> Self {
        Self::new()
    }
}

impl Portfolio {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Vec::<(String, i32)>::new(),
        }
    }

    pub fn symbol_iter(&self) -> impl Iterator<Item = &str> {
        self.data.iter().map(|p| p.0.as_str())
    }

    #[must_use]
    pub fn amount(&self, symbol: &str) -> i32 {
        match self.data.iter().find(|t| t.0 == symbol) {
            Some(n) => n.1,
            None => 0,
        }
    }

    pub fn add_transaction(&mut self, t: &MyTransaction) {
        match self.data.iter_mut().find(|p| p.0 == t.symbol) {
            Some(p) => p.1 += t.number,
            None => self.data.push((t.symbol.clone(), t.number)),
        }
    }
}

#[allow(dead_code)]
impl fmt::Display for Portfolio {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let entries: Vec<String> = self
            .symbol_iter()
            .map(|s| format!("{}: {}", s, self.amount(s)))
            .collect();
        write!(f, "{}", entries.join(","))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MyTransaction {
    // date;symbol;number;price;commision;currency
    pub date: NaiveDate,
    pub symbol: String,
    pub number: i32,
    pub price: Decimal,
    pub commision: Decimal,
    pub currency: String,
}

impl fmt::Display for MyTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {}: {} * {} {}",
            self.date, self.symbol, self.number, self.price, self.currency
        )
    }
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
    /// - Has bad haders
    /// - Hit format errors.
    pub fn try_from_reader<R: Read>(rd: R) -> csv::Result<Self> {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .from_reader(rd);

        // Validate headers
        let headers = rdr.headers()?;
        let expected = ["date", "symbol", "number", "price", "commision", "currency"];
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

    #[must_use]
    pub fn my_days_iter(&self) -> Box<dyn Iterator<Item = NaiveDate> + '_> {
        match (self.first_date(), self.last_date()) {
            (Some(start), Some(end)) => Box::new(start.iter_days().take_while(move |d| d <= end)),
            _ => Box::new(std::iter::empty()),
        }
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &MyTransaction> + '_ {
        self.p.iter()
    }

    #[must_use]
    pub fn first_date(&self) -> Option<&NaiveDate> {
        self.p.iter().min_by_key(|x| x.date).map(|x| &x.date)
    }

    #[must_use]
    pub fn last_date(&self) -> Option<&NaiveDate> {
        self.p.iter().max_by_key(|x| x.date).map(|x| &x.date)
    }

    pub fn trans_by_date<'a>(
        &'a self,
        d: &'a NaiveDate,
    ) -> impl Iterator<Item = &'a MyTransaction> {
        self.p.iter().filter(move |x| &x.date == d)
    }
}

#[cfg(test)]
mod test {
    use crate::transactions::{Portfolio, Transactions};
    use chrono::NaiveDate;
    use indoc::indoc;
    use std::io::{Cursor, Read};

    #[test]
    fn test_from_csv() {
        let test_rd = Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
        "});

        let sut = Transactions::try_from_reader(test_rd).unwrap();
        sut.iter().next().unwrap();
    }

    #[test]
    fn test_date_empty() {
        let sut = Transactions::new();
        assert!(sut.first_date().is_none());
    }

    #[test]
    fn test_the_first1() {
        let sut = Transactions::try_from_reader(test_rd()).unwrap();
        assert_eq!(
            sut.first_date().unwrap(),
            &NaiveDate::from_ymd_opt(2000, 01, 01).unwrap()
        )
    }

    #[test]
    fn test_the_last1() {
        let sut = Transactions::try_from_reader(test_rd()).unwrap();
        assert_eq!(
            sut.last_date().unwrap(),
            &NaiveDate::from_ymd_opt(2000, 01, 02).unwrap()
        )
    }

    #[test]
    fn test_by_date() {
        let sut = Transactions::try_from_reader(test_rd()).unwrap();
        let d = NaiveDate::from_ymd_opt(2000, 01, 01).unwrap();
        let mut i = sut.trans_by_date(&d);
        let v = i.next().unwrap();
        assert_eq!(v.date, d);
        assert!(i.next().is_none());
    }

    fn test_rd() -> impl Read {
        Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
            2000-01-02;BAZ;1;42.42;4.2;QUX
        "})
    }

    #[test]
    fn test_days_iter() {
        let sut = Transactions::try_from_reader(Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
            2000-01-03;BAZ;1;42.42;4.2;QUX
        "}))
        .unwrap();
        let mut i = sut.my_days_iter();
        assert_eq!(
            i.next().unwrap(),
            NaiveDate::from_ymd_opt(2000, 01, 01).unwrap()
        );
        assert_eq!(
            i.next().unwrap(),
            NaiveDate::from_ymd_opt(2000, 01, 02).unwrap()
        );
        assert_eq!(
            i.next().unwrap(),
            NaiveDate::from_ymd_opt(2000, 01, 03).unwrap()
        );
    }

    #[test]
    fn test_portfolio_emty() {
        let sut = Portfolio::new();
        assert_eq!(sut.amount("anythin"), 0)
    }

    #[test]
    fn portfolio1() {
        let t = Transactions::try_from_reader(Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
        "}))
        .unwrap();
        let mut ti = t.iter();
        let t1 = ti.next().unwrap();
        let mut sut = Portfolio::new();
        sut.add_transaction(t1);
        assert_eq!(sut.amount("FOO"), 1);
    }
    #[test]
    fn portfolio2() {
        let mut sut = Portfolio::new();
        let t = Transactions::try_from_reader(Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
            2000-01-01;BAR;1;42.42;4.2;BAR
        "}))
        .unwrap();
        t.iter().for_each(|t| sut.add_transaction(t));

        assert_eq!(sut.amount("FOO"), 1);
        assert_eq!(sut.amount("BAR"), 1);
    }

    #[test]
    fn portfolio_same() {
        let mut sut = Portfolio::new();
        let t = Transactions::try_from_reader(Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
            2000-01-01;FOO;1;42.42;4.2;BAR
        "}))
        .unwrap();
        t.iter().for_each(|t| sut.add_transaction(t));

        assert_eq!(sut.amount("FOO"), 2);
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
            date,symbol,number,price,commision,currency
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
