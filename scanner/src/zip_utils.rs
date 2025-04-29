use anyhow::{Context, Result};
use std::io::{Read, Seek};
use zip::read::ZipArchive;

/// Extract and read a specific file from a ZIP archive as a string
pub fn read_file_as_string<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Result<String> {
    let mut file = archive
        .by_name(path)
        .with_context(|| format!("Failed to find file in archive: {}", path))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .with_context(|| format!("Failed to read file from archive: {}", path))?;

    Ok(content)
}

/// Find the dist-info directory in a wheel archive
pub fn find_dist_info_dir<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let filename = file.name();
            if filename.ends_with(".dist-info/") {
                return Some(filename.to_string());
            }
        }
    }

    None
}
