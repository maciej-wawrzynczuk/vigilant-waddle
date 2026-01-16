use walkdir::WalkDir;
use std::path::PathBuf;

pub fn list_files() -> impl Iterator<Item = PathBuf> {
    WalkDir::new(crate::DATA_BASE)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_owned())
}
