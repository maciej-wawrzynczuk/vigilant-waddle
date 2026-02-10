use std::{
    fs::File,
    io::{Cursor, Read},
};

use crate::stooq_download::raw_filename;
use polars::prelude::*;

pub fn load_csv(symbol: &str) -> PolarsResult<DataFrame> {
    let filename = raw_filename(symbol);
    let mut f = File::open(filename)?;
    csv_from_reader(&mut f)
}

pub fn csv_from_reader<T: Read>(r: &mut T) -> PolarsResult<DataFrame> {
    let mut schema = Schema::default();
    schema.insert("date".into(), DataType::Date);
    schema.insert("open".into(), DataType::Float32);
    schema.insert("high".into(), DataType::Float32);
    schema.insert("low".into(), DataType::Float32);
    schema.insert("close".into(), DataType::Float32);
    schema.insert("volume".into(), DataType::Float32);

    let mut b = Vec::new();
    r.read_to_end(&mut b)?;
    let c = Cursor::new(b);
    let df = CsvReadOptions::default()
        .with_schema(Some(Arc::new(schema)))
        .into_reader_with_file_handle(c)
        .finish()?;
    Ok(df)
}

#[cfg(test)]
mod test {
    use crate::dataframe::csv_from_reader;
    use indoc::indoc;
    use std::io::Cursor;

    #[test]
    fn test_read() {
        let mut test_csv = a_stooq_csv();
        csv_from_reader(&mut test_csv).unwrap();
    }

    #[test]
    fn test_schema() {
        let mut test_csv = a_stooq_csv();
        let df = csv_from_reader(&mut test_csv).unwrap();
        let names = df.get_column_names_str();
        assert_eq!(names[0], "date");
    }

    fn a_stooq_csv() -> Cursor<&'static str> {
        Cursor::new(indoc! {"
            Date,Open,High,Low,Close,Volume
            1962-01-02,5.0461,5.0461,4.98716,4.98716,593562.95523744
            1962-01-03,4.98716,5.03292,4.98716,5.03292,445175.03427694
            1962-01-04,5.03292,5.03292,4.98052,4.98052,399513.58667937
            1962-01-05,4.97389,4.97389,4.87511,4.88166,559321.48056467
            1962-01-08,4.88166,4.88166,4.75059,4.78972,833273.77139308
            1962-01-09,4.81618,4.9082,4.81618,4.84867,753373.92313968
            1962-01-10,4.85531,4.88166,4.85531,4.85531,456586.80982324
            1962-01-11,4.86878,4.9082,4.86878,4.9082,490831.35851294
            1962-01-12,4.92119,4.95417,4.92119,4.92119,673473.05021397
            "})
    }
}
