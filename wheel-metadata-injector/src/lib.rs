use indexmap::IndexMap;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use time::OffsetDateTime;

use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;

use hex::encode;
use sha2::{Digest, Sha256};
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

pub const ENV_WHITELIST: &[&str] = &[
    // PyTorch/CUDA build info
    "TORCH_CUDA_ARCH_LIST",
    "CUDA_VERSION",
    "CUDA_HOME",
    "CUDNN_VERSION",
    "BUILD_TYPE",
    "PYTORCH_BUILD_VERSION",
    "PYTORCH_BUILD_NUMBER",
    "CMAKE_ARGS",
    "EXTRA_CAFFE2_CMAKE_FLAGS",
    // ROCm build info
    "PYTORCH_ROCM_ARCH",
    "GPU_ARCHS",
    // GitHub Actions CI variables
    "GITHUB_SHA",
    "GITHUB_REPOSITORY",
    "GITHUB_WORKFLOW",
    "GITHUB_JOB",
    "GITHUB_RUN_ID",
    "RUNNER_OS",
    "RUNNER_ARCH",
    // Build system info
    "CMAKE_BUILD_TYPE",
    "PYTHON_VERSION",
    "SETUPTOOLS_VERSION",
    "PIP_VERSION",
    "CC",
    "CXX",
    "CFLAGS",
    "CXXFLAGS",
    "LDFLAGS",
];

// PEP 658 specifics - filename for build environment metadata
pub const BUILD_ENV_FILENAME: &str = "WHEEL.metadata";

#[pyclass]
pub struct WheelInfo {
    #[pyo3(get)]
    pub dist_info_dir: String,
    #[pyo3(get)]
    pub metadata_path: String,
}

#[pyfunction]
fn get_env_vars_from_comma_list(comma_list: String) -> PyResult<Vec<(String, String)>> {
    Ok(collect_env_vars_from_comma_list(&comma_list))
}

#[pyfunction]
fn process_wheel_with_env_vars(
    wheel_path: String,
    env_vars: String,
    output_path: Option<String>,
) -> PyResult<String> {
    let output_path = output_path.unwrap_or_else(|| wheel_path.clone());

    let env_vars = collect_env_vars_from_comma_list(&env_vars);

    match internal_process_wheel(&wheel_path, &output_path, &env_vars) {
        Ok(_) => Ok(output_path),
        Err(e) => Err(PyValueError::new_err(format!(
            "Error processing wheel: {}",
            e
        ))),
    }
}

#[pymodule]
fn _wheel_metadata_injector(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<WheelInfo>()?;
    m.add_function(wrap_pyfunction!(process_wheel, m)?)?;
    m.add_function(wrap_pyfunction!(process_wheel_with_env_file, m)?)?;
    m.add_function(wrap_pyfunction!(process_wheel_with_env_vars, m)?)?;
    m.add_function(wrap_pyfunction!(get_whitelisted_env_vars, m)?)?;
    m.add_function(wrap_pyfunction!(get_whitelisted_env_vars_with_file, m)?)?;
    m.add_function(wrap_pyfunction!(get_env_vars_from_comma_list, m)?)?;
    Ok(())
}

#[pyfunction]
fn process_wheel(wheel_path: String, output_path: Option<String>) -> PyResult<String> {
    let output_path = output_path.unwrap_or_else(|| wheel_path.clone());

    let env_vars = collect_whitelisted_env_vars();

    match internal_process_wheel(&wheel_path, &output_path, &env_vars) {
        Ok(_) => Ok(output_path),
        Err(e) => Err(PyValueError::new_err(format!(
            "Error processing wheel: {}",
            e
        ))),
    }
}

#[pyfunction]
fn process_wheel_with_env_file(
    wheel_path: String,
    env_file: String,
    output_path: Option<String>,
) -> PyResult<String> {
    let output_path = output_path.unwrap_or_else(|| wheel_path.clone());

    let env_vars = collect_whitelisted_env_vars_with_file(Some(&env_file));

    match internal_process_wheel(&wheel_path, &output_path, &env_vars) {
        Ok(_) => Ok(output_path),
        Err(e) => Err(PyValueError::new_err(format!(
            "Error processing wheel: {}",
            e
        ))),
    }
}

#[pyfunction]
fn get_whitelisted_env_vars() -> PyResult<Vec<(String, String)>> {
    Ok(collect_whitelisted_env_vars())
}

#[pyfunction]
fn get_whitelisted_env_vars_with_file(env_file: String) -> PyResult<Vec<(String, String)>> {
    Ok(collect_whitelisted_env_vars_with_file(Some(&env_file)))
}

pub fn internal_process_wheel(
    wheel_path: &str,
    output_path: &str,
    env_vars: &[(String, String)],
) -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let temp_dir_path = temp_dir.path();

    let wheel_info = unpack_wheel(wheel_path, temp_dir_path)?;
    let build_env_path = temp_dir_path
        .join(&wheel_info.dist_info_dir)
        .join(BUILD_ENV_FILENAME);

    let git = get_repository_info();
    let automation = get_pipeline_info();

    create_build_env_file(&build_env_path, env_vars, git, automation)?;
    update_record_file(temp_dir_path, &wheel_info, &build_env_path)?;
    repack_wheel(temp_dir_path, output_path)?;

    Ok(())
}

pub fn get_pipeline_info() -> Option<AutomationInfo> {
    
    // Check if running in GitHub Actions
    if env::var("GITHUB_ACTIONS").is_ok() {
        let run_id = env::var("GITHUB_RUN_ID").ok();
        let workflow_name = env::var("GITHUB_WORKFLOW").ok();
        let workflow_sha = env::var("GITHUB_WORKFLOW_SHA").ok();
        let job_name = env::var("GITHUB_JOB").ok();
        let runner_name = env::var("RUNNER_NAME").ok();

        return Some(AutomationInfo {
            actions_info: Some(ActionsInfo {
                run_id,
                workflow_name,
                workflow_sha,
                job_name,
                runner_name,
            }),
        });
    }




    None
}

pub fn get_repository_info() -> Option<RepositoryInfo> {
    // Get git information from the current directory using libgit2
    let repo = match git2::Repository::discover(".") {
        Ok(repo) => repo,
        Err(_) => return None,
    };

    let remotes = match repo.remotes() {
        Ok(remotes) => remotes,
        Err(_) => return None,
    };
    let mut remote_url = None;
    // TODO: Handle multiple remotes, maybe just use the first one
    // or the one that matches a specific pattern, or list them all
    for remote in remotes.iter().flatten() {
        match repo.find_remote(remote) {
            Ok(remote) => remote_url = remote.url().map(|s| s.to_string()),
            Err(_) => return None,
        }
    }

    let commit = match repo.head() {
        Ok(head) => head.peel_to_commit().ok(),
        Err(_) => None,
    };

    let commit = match commit {
        Some(commit) => commit.id().to_string(),
        None => return None,
    };

    Some(RepositoryInfo {
        url: remote_url,
        commit,
    })
}

pub fn read_vars_list_from_file(file_path: &str) -> io::Result<Vec<String>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut var_names = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        var_names.push(trimmed.to_string());
    }

    Ok(var_names)
}

pub fn collect_whitelisted_env_vars() -> Vec<(String, String)> {
    collect_env_vars_from_list(ENV_WHITELIST.iter().map(|&s| s.to_string()).collect())
}

pub fn collect_whitelisted_env_vars_with_file(vars_file: Option<&str>) -> Vec<(String, String)> {
    match vars_file {
        Some(file_path) => {
            match read_vars_list_from_file(file_path) {
                Ok(var_list) => collect_env_vars_from_list(var_list),
                Err(_) => {
                    // Fall back to default whitelist on file error
                    collect_whitelisted_env_vars()
                }
            }
        }
        None => collect_whitelisted_env_vars(),
    }
}

pub fn collect_env_vars_from_comma_list(comma_list: &str) -> Vec<(String, String)> {
    let var_names: Vec<String> = comma_list
        .split(',')
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();

    collect_env_vars_from_list(var_names)
}

fn collect_env_vars_from_list(var_names: Vec<String>) -> Vec<(String, String)> {
    let mut env_vars = Vec::new();

    for var_name in var_names {
        if let Ok(value) = env::var(&var_name) {
            env_vars.push((var_name, value));
        }
    }

    env_vars
}

pub fn unpack_wheel(wheel_path: &str, temp_dir: &Path) -> io::Result<WheelInfo> {
    let file = File::open(wheel_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut dist_info_dir = None;

    // First pass: find the dist-info directory name
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let file_path = file.name();

        if let Some(idx) = file_path.find(".dist-info/") {
            let dir_name = &file_path[..idx + 10]; // +10 to include ".dist-info"
            dist_info_dir = Some(dir_name.to_string());
            break;
        }
    }

    let dist_info_dir = match dist_info_dir {
        Some(dir) => dir,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No .dist-info directory found in wheel",
            ));
        }
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = file.name();

        let output_path = temp_dir.join(file_path);

        if file.is_dir() {
            fs::create_dir_all(&output_path)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output_file = File::create(&output_path)?;
            io::copy(&mut file, &mut output_file)?;
        }
    }

    let metadata_path = format!("{}/METADATA", dist_info_dir);

    Ok(WheelInfo {
        dist_info_dir,
        metadata_path,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildEnvMetadata {
    #[serde(serialize_with = "time::serde::iso8601::serialize")]
    build_time: OffsetDateTime,
    git: Option<RepositoryInfo>,
    #[serde(rename = "env")]
    env_vars: IndexMap<String, String>,
    automation: Option<AutomationInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// The git remote URL of the repository.
    #[serde(rename = "url")]
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    /// The commit hash of the repository at the time of wheel creation.
    #[serde(rename = "commit")]
    commit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutomationInfo {
    /// Information specific to github actions.
    #[serde(flatten)]
    actions_info: Option<ActionsInfo>,

}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionsInfo {
    /// The GitHub Actions run ID.
    #[serde(rename = "run_id")]
    run_id: Option<String>,
    /// The GitHub Actions workflow name.
    #[serde(rename = "workflow_name")]
    workflow_name: Option<String>,
    /// The GitHub Actions workflow SHA.
    #[serde(rename = "workflow_sha")]
    workflow_sha: Option<String>,
    /// The GitHub Actions job name.
    #[serde(rename = "job_name")]
    job_name: Option<String>,
    /// Runner name.
    #[serde(rename = "runner_name")]
    runner_name: Option<String>,
}

pub fn create_build_env_file(
    build_env_path: &Path,
    env_vars: &[(String, String)],
    git: Option<RepositoryInfo>,
    automation: Option<AutomationInfo>,
) -> anyhow::Result<()> {
    let mut content = String::new();

    content.push_str("# Build environment variables captured during wheel creation\n");
    content.push_str(
        "# This file adheres to PEP 658 and contains whitelisted environment variables\n\n",
    );

    let env_vars: IndexMap<String, String> = env_vars
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let bem = BuildEnvMetadata {
        env_vars,
        build_time: OffsetDateTime::now_utc(),
        git,
        automation,
    };

    // Serialize the BuildEnvMetadata struct to TOML format
    content.push_str(&toml::to_string(&bem)?);

    println!("Content: {}", content);

    let mut file = File::create(build_env_path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

fn calculate_file_hash(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let hash = hasher.finalize();

    Ok(format!("sha256={}", encode(hash)))
}

pub fn update_record_file(
    temp_dir: &Path,
    wheel_info: &WheelInfo,
    build_env_path: &Path,
) -> io::Result<()> {
    let record_path = temp_dir.join(&wheel_info.dist_info_dir).join("RECORD");

    let mut record_content = String::new();
    File::open(&record_path)?.read_to_string(&mut record_content)?;

    let hash = calculate_file_hash(build_env_path)?;
    let size = fs::metadata(build_env_path)?.len();

    let rel_path = format!("{}/{}", wheel_info.dist_info_dir, BUILD_ENV_FILENAME);
    let record_entry = format!("{},{},{}\n", rel_path, hash, size);

    record_content.push_str(&record_entry);

    let mut file = File::create(record_path)?;
    file.write_all(record_content.as_bytes())?;

    Ok(())
}

pub fn repack_wheel(temp_dir: &Path, output_path: &str) -> io::Result<()> {
    let output_file = File::create(output_path)?;
    let mut zip = ZipWriter::new(output_file);

    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut paths: Vec<PathBuf> = Vec::new();
    collect_files(temp_dir, &mut paths)?;

    paths.sort();

    for path in paths {
        let rel_path = path.strip_prefix(temp_dir).unwrap();
        let rel_path_str = rel_path.to_string_lossy().replace("\\", "/");

        if path.is_file() {
            zip.start_file(&rel_path_str, options)?;
            let mut file = File::open(&path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if path.is_dir() {
            let dir_path = format!("{}/", rel_path_str);
            zip.add_directory(&dir_path, options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

fn collect_files(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            paths.push(path.clone());

            if path.is_dir() {
                collect_files(&path, paths)?;
            }
        }
    }

    Ok(())
}
