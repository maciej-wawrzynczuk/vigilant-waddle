use serde::Deserialize;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::io::Read;
use anyhow::Result;


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
}

#[cfg(test)]
mod test {
    use indoc::indoc;
    use std::io::Cursor;

    use crate::transactions::Transactions;
    #[test]
    fn test_from_csv() {
        let test_rd = Cursor::new(indoc! {"
            date;symbol;number;price;commision;currency
            2000-01-01;FOO;1;42.42;4.2;BAR
        "});

        let sut = Transactions::from_reader(test_rd);
        sut.into_iter().next().unwrap();

    }
}
