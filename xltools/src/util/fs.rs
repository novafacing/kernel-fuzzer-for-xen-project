use std::{
    fs::{copy, create_dir_all},
    path::PathBuf,
};

use anyhow::Result;
use walkdir::WalkDir;

/// Copy all files and directories in a directory to another directory
pub fn copy_dir(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let path = entry.path();
        let dest_path = dest.join(path.strip_prefix(src)?);
        if metadata.is_file() {
            copy(path, &dest_path)?;
        } else if metadata.is_dir() {
            create_dir_all(dest_path)?;
        }
    }
    Ok(())
}
