use crate::stooq_download::raw_filename;
use polars::prelude::*;


pub fn load_csv(symbol: &str) ->PolarsResult<DataFrame>{
    let f = raw_filename(symbol);
    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some(f))?
        .finish()?;

    Ok(df)
}
