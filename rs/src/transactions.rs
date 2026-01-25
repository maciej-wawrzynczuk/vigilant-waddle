use serde::Deserialize;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::io::Read;


#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct MyTransaction {
    // date;symbol;number;price;commision;currency
    pub date: NaiveDate,
    pub symbol: String,
    pub number: i32,
    pub price: Decimal,
    pub commision: Decimal,
    pub currency: String,
}

#[derive(Debug)]
pub struct Transactions {
    p: Vec<MyTransaction>
}

impl Transactions {
    pub fn new() -> Self {
        Self{
            p: Vec::new()
        }
    }

    pub fn from_reader<R: Read>(rd: R) -> csv::Result<Self> {

        let v = csv::ReaderBuilder::new()
            .delimiter(b';')                                                                                                                                          
            .has_headers(true)                                                                                                                                        
            .from_reader(rd)                                                                                                                                          
            .into_deserialize()
            .collect::<csv::Result<Vec<MyTransaction>>>()?;

        Ok(Self { p: v })
    }

    pub fn into_iter(&self) -> impl Iterator<Item = &MyTransaction> + '_ {
        self.p.iter()
    }

    pub fn first_date(&self) -> Option<&NaiveDate> {
        self.p.iter()
            .min_by_key(|x| x.date)
            .map(|x| &x.date)
    }

    pub fn last_date(&self) -> Option<&NaiveDate> {
        self.p.iter()
            .max_by_key(|x| x.date)
            .map(|x| &x.date)
    }

    pub fn trans_by_date<'a>(&'a self, d: &'a NaiveDate) -> impl Iterator<Item = &'a MyTransaction> {
        self.p.iter()
            .filter(move |x| &x.date == d)
    }
}

#[cfg(test)]
mod test {
    use chrono::NaiveDate;
    use indoc::indoc;
    use std::io::{Cursor, Read};

    use crate::transactions::Transactions;
    #[test]
    fn test_from_csv() {
        let test_rd = Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
        "});

        let sut = Transactions::from_reader(test_rd).unwrap();
        sut.into_iter().next().unwrap();

    }

    #[test]
    fn test_date_empty() {
        let sut = Transactions::new();
        assert!(sut.first_date().is_none());
    }

    #[test]
    fn test_the_first1() {
        let sut = Transactions::from_reader(test_rd()).unwrap();
        assert_eq!(sut.first_date().unwrap(), &NaiveDate::from_ymd_opt(2000, 01, 01).unwrap())
    }

    #[test]
    fn test_the_last1() {
        let sut = Transactions::from_reader(test_rd()).unwrap();
        assert_eq!(sut.last_date().unwrap(), &NaiveDate::from_ymd_opt(2000, 01, 02).unwrap())
    }

    #[test]
    fn test_by_date() {
        let sut = Transactions::from_reader(test_rd()).unwrap();
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
}
