use std::path::{PathBuf, Path};
use chrono::Utc;
use crate::paths;

pub trait DbConfig {
    fn db_base(&self) -> &str;
}


pub type Fields = Vec<(String, String)>;

pub fn mk_hive(fields: Fields) -> PathBuf {
    let mut p = PathBuf::new();
    for (k,v) in fields {
        p.push(format!("{k}={v}"));
    }
    p
}
pub fn full_path<T: DbConfig>(p: &Path, c: &T) -> PathBuf {
    PathBuf::from(c.db_base())
        .join(p)
}

pub fn part_path(source: &str, symbol: &str) -> PathBuf {
    let now = Utc::now();
    let f = vec![
        ("symbol".to_string(), symbol.to_string()),
        ("year".to_string(), now.format("%Y").to_string()),
        ("month".to_string(), now.format("%m").to_string()),
        ("day".to_string(), now.format("%d").to_string())
    ];
    let ts = now.format("%Y%m%dT%H%M%SZ").to_string();
    let h_part = paths::mk_hive(f);

    PathBuf::new()
        .join("raw")
        .join(source)
        .join(h_part)
        .join(format!("{ts}.csv"))

    //format!("{DATA_BASE}/raw/stooq/{symbol}/{year}/{month}/{day}/{ts}.csv")
}

#[cfg(test)]
mod test {
    use crate::paths::{mk_hive, part_path};
    use std::path::Component;

    #[test]
    fn test_mk_hive() {
        let test_data = vec![
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string())
        ];
        let expected = "foo=bar/baz=qux";
        assert!(mk_hive(test_data).to_str().unwrap() == expected);
    }

    #[test]
    fn test_path() {
        let p = part_path("stooq", "foo");
        let parts: Vec<_> = p.components().collect();
        let p_str = p.to_str().unwrap();
        println!("Returned {p_str}");
        assert_eq!(parts[0], Component::Normal("raw".as_ref()));
        assert_eq!(parts[1], Component::Normal("stooq".as_ref()));
    }
}
