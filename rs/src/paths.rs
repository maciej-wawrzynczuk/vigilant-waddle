use std::path::PathBuf;

pub trait DbConfig {
    fn db_base(&self) -> &str;
}

pub struct PathMan<'a, T: DbConfig> {
    config: &'a T
}

impl<'a, T: DbConfig> PathMan<'a, T> {
    pub fn new(c: &'a T) -> Self {
        Self {config: c}
    }

    pub fn patch_from_tags(&self, tags: &[&str]) -> PathBuf {
        let s: PathBuf = tags.iter().collect();
        PathBuf::from(self.config.db_base())
            .join(s)
    }
}

#[cfg(test)]
mod test {
    use std::path::Component;

    struct FakeConfig;

    impl crate::paths::DbConfig for FakeConfig {
        fn db_base(&self) -> &str {
            "/a_folder"
        }
    }

    #[test]
    fn test_pm_tags() {
        let f = FakeConfig;
        let fc = crate::paths::PathMan::new(&f);
        let p = fc.patch_from_tags(&["foo", "bar", "baz"]);
        let parts: Vec<_> = p.components().collect();
        let p_str = p.to_str().unwrap();
        println!("Returned {p_str}");
        assert_eq!(parts[0], Component::RootDir);
        assert_eq!(parts[1], Component::Normal("a_folder".as_ref()));
        assert_eq!(parts[2], Component::Normal("foo".as_ref()));
        assert_eq!(parts[3], Component::Normal("bar".as_ref()));
        assert_eq!(parts[4], Component::Normal("baz".as_ref()));
    }
}
