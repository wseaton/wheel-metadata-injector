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
            // look for files within .dist-info directories
            if let Some(dist_info_pos) = filename.find(".dist-info/") {
                let dist_info_dir = &filename[..dist_info_pos + 10]; // +10 for ".dist-info"
                return Some(format!("{}/", dist_info_dir));
            }
        }
    }

    None
}

/// Parse wheel METADATA file and extract dependency version
pub fn extract_dependency_version<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    dist_info_dir: &str,
    dependency_name: &str,
) -> Result<Option<String>> {
    let metadata_path = format!("{}METADATA", dist_info_dir);

    let metadata_content = read_file_as_string(archive, &metadata_path)
        .with_context(|| format!("Failed to read METADATA file: {}", metadata_path))?;

    for line in metadata_content.lines() {
        if line.starts_with("Requires-Dist:") {
            let requirement = line.strip_prefix("Requires-Dist:").unwrap().trim();

            if let Some(parsed_req) = parse_requirement(requirement) {
                if parsed_req.name.to_lowercase() == dependency_name.to_lowercase() {
                    return Ok(Some(parsed_req.version_spec));
                }
            }
        }
    }

    Ok(None)
}

/// Simple requirement parser that extracts name and version spec
struct ParsedRequirement {
    name: String,
    version_spec: String,
}

fn parse_requirement(requirement: &str) -> Option<ParsedRequirement> {
    let requirement = requirement.trim();

    // handle conditional dependencies like "torch>=1.0; extra == 'torch'"
    let core_req = if let Some(semicolon_pos) = requirement.find(';') {
        requirement[..semicolon_pos].trim()
    } else {
        requirement
    };

    // find version operators: ==, >=, <=, >, <, !=, ~=
    let version_operators = [">=", "<=", "==", "!=", "~=", ">", "<"];

    for op in &version_operators {
        if let Some(op_pos) = core_req.find(op) {
            let name = core_req[..op_pos].trim().to_string();
            let version_spec = core_req[op_pos..].trim().to_string();

            return Some(ParsedRequirement { name, version_spec });
        }
    }

    // if no version spec found, return just the name with empty version
    Some(ParsedRequirement {
        name: core_req.to_string(),
        version_spec: String::new(),
    })
}
