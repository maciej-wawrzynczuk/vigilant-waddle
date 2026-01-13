use std::path::PathBuf;

pub type Fields = Vec<(String, String)>;

pub fn mk_hive(fields: Fields) -> PathBuf {
    let mut p = PathBuf::new();
    for (k,v) in fields {
        p.push(format!("{k}={v}"));
    }

    p
}

#[cfg(test)]
mod test {
    use crate::hivepart::mk_hive;

    #[test]
    fn test_mk_hive() {
        let test_data = vec![
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string())
        ];
        let expected = "foo=bar/baz=qux";
        assert!(mk_hive(test_data).to_str().unwrap() == expected);
    }
}
