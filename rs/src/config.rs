use crate::raw::stooq::DbConfig;

pub struct SillyConfig;

impl DbConfig for SillyConfig {
    fn db_base(&self) -> &str {
        crate::DATA_BASE
    }
}
