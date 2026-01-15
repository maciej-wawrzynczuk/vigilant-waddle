use walkdir::WalkDir;
use anyhow::Result;

pub fn list_files() -> Result<()> {
    for e in WalkDir::new(crate::DATA_BASE).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
            println!("{}", e.path().display());
    }
    Ok(())
}
