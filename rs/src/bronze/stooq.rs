use walkdir::WalkDir;
use anyhow::Result;

pub fn list_files() -> Result<()> {
    for e in WalkDir::new(crate::DATA_BASE) {
        println!("{}", e?.path().display());
    }
    Ok(())
}
