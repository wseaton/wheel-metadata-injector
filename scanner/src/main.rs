use anyhow::{Context, Result};
use bytes::Bytes;
use clap::Parser;

use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;

use std::io::{Read, Seek, SeekFrom};
use tokio::task;
use tracing_subscriber::EnvFilter;
use zip::read::ZipArchive;

mod zip_utils;

const BUILD_ENV_FILENAME: &str = "WHEEL.metadata";

/// Wheel Metadata Scanner - Extract build metadata from Python wheel packages
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to wheel file or GCP URL (gs://bucket/path/to/wheel)
    #[clap(value_parser)]
    wheel_path: String,

    /// Extract all wheel metadata, not just the build environment
    #[clap(short, long, default_value = "false")]
    all_metadata: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .from_env_lossy(),
        )
        .init();
    let args = Args::parse();

    if args.wheel_path.starts_with("gs://") {
        extract_from_cloud(&args.wheel_path, args.all_metadata).await
    } else {
        extract_from_local_file(&args.wheel_path, args.all_metadata).await
    }
}

/// Extract metadata from a local wheel file
#[tracing::instrument(skip_all)]
async fn extract_from_local_file(wheel_path: &str, all_metadata: bool) -> Result<()> {
    tracing::info!("Reading local wheel file: {}", wheel_path);

    let file = std::fs::File::open(wheel_path)
        .with_context(|| format!("Failed to open wheel file: {}", wheel_path))?;

    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("Failed to open ZIP archive: {}", wheel_path))?;

    let dist_info_dir = zip_utils::find_dist_info_dir(&mut archive)
        .with_context(|| "Failed to find .dist-info directory in wheel")?;

    tracing::info!("Found dist-info directory: {}", dist_info_dir);

    let metadata_path = format!("{}{}", dist_info_dir, BUILD_ENV_FILENAME);

    let has_metadata = archive
        .by_name(&metadata_path)
        .map(|file| {
            tracing::info!("Found {} ({} bytes)", BUILD_ENV_FILENAME, file.size());
            true
        })
        .unwrap_or(false);

    if has_metadata {
        let metadata_content = zip_utils::read_file_as_string(&mut archive, &metadata_path)
            .with_context(|| format!("Failed to read {}", BUILD_ENV_FILENAME))?;

        tracing::info!("=== Build Environment Metadata ===");
        println!("{}", metadata_content);
    } else {
        tracing::info!("No build environment metadata (WHEEL.metadata) found in wheel!");
    }

    if all_metadata {
        let pkg_metadata_path = format!("{}METADATA", dist_info_dir);

        let has_pkg_metadata = archive
            .by_name(&pkg_metadata_path)
            .map(|file| {
                tracing::info!("Found METADATA ({} bytes)", file.size());
                true
            })
            .unwrap_or(false);

        if has_pkg_metadata {
            let pkg_metadata_content =
                zip_utils::read_file_as_string(&mut archive, &pkg_metadata_path)
                    .with_context(|| "Failed to read METADATA")?;

            tracing::info!("\n=== Package Metadata ===");
            println!("{}", pkg_metadata_content);
        } else {
            tracing::info!("\nNo package METADATA file found in wheel!");
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn extract_from_cloud(uri: &str, all_metadata: bool) -> Result<()> {
    let uri = uri
        .strip_prefix("gs://")
        .context("URI must start with gs://")?;
    let parts: Vec<&str> = uri.splitn(2, '/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid GCS URI format. Expected gs://bucket/path/to/object");
    }

    let bucket = parts[0];
    let object_path = parts[1];

    tracing::info!("Connecting to GCS bucket: {}", bucket);

    let config = ClientConfig::default().with_auth().await?;
    let client = Client::new(config);

    let gcs_reader =
        GcsRangedReader::new(client, bucket.to_string(), object_path.to_string()).await?;

    let mut archive = ZipArchive::new(gcs_reader).context("Failed to open ZIP archive from GCS")?;

    let dist_info_dir = zip_utils::find_dist_info_dir(&mut archive)
        .with_context(|| "Failed to find .dist-info directory in wheel")?;

    tracing::info!("Found dist-info directory: {}", dist_info_dir);

    let metadata_path = format!("{}{}", dist_info_dir, BUILD_ENV_FILENAME);

    let has_metadata = archive
        .by_name(&metadata_path)
        .map(|file| {
            tracing::info!("Found {} ({} bytes)", BUILD_ENV_FILENAME, file.size());
            true
        })
        .unwrap_or(false);

    if has_metadata {
        let metadata_content = zip_utils::read_file_as_string(&mut archive, &metadata_path)
            .with_context(|| format!("Failed to read {}", BUILD_ENV_FILENAME))?;

        tracing::info!("=== Build Environment Metadata ===");
        println!("{}", metadata_content);
    } else {
        tracing::info!("No build environment metadata (WHEEL.metadata) found in wheel!");
    }

    if all_metadata {
        let pkg_metadata_path = format!("{}METADATA", dist_info_dir);

        let has_pkg_metadata = archive
            .by_name(&pkg_metadata_path)
            .map(|file| {
                tracing::info!("Found METADATA ({} bytes)", file.size());
                true
            })
            .unwrap_or(false);

        if has_pkg_metadata {
            let pkg_metadata_content =
                zip_utils::read_file_as_string(&mut archive, &pkg_metadata_path)
                    .with_context(|| "Failed to read METADATA")?;

            tracing::info!("=== Package Metadata ===");
            println!("{}", pkg_metadata_content);
        } else {
            tracing::info!("\nNo package METADATA file found in wheel!");
        }
    }

    Ok(())
}

/// A struct that implements Read and Seek for Google Cloud Storage with ranged reads
struct GcsRangedReader {
    client: Client,
    bucket: String,
    object: String,
    position: u64,
    total_size: u64,
    cache: Option<(u64, u64, Bytes)>, // (start, end, data)
    total_bytes_read: u64,
}

impl GcsRangedReader {
    /// Create a new GcsRangedReader
    async fn new(client: Client, bucket: String, object: String) -> Result<Self> {
        let req = GetObjectRequest {
            bucket: bucket.clone(),
            object: object.clone(),
            ..Default::default()
        };

        let metadata = client.get_object(&req).await?;
        let total_size = metadata.size.max(0) as u64;

        tracing::info!("Object size: {} bytes", total_size);

        Ok(Self {
            client,
            bucket,
            object,
            position: 0,
            total_size,
            cache: None,
            total_bytes_read: 0,
        })
    }

    /// Fetch a range of bytes from Google Cloud Storage
    #[tracing::instrument(skip(self))]
    fn fetch_range(&self, start: u64, end: u64) -> std::io::Result<Bytes> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let object = self.object.clone();

        // Use block_in_place to avoid deadlocks
        task::block_in_place(|| {
            // Create an isolated runtime for this blocking operation
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            rt.block_on(async move {
                let req = GetObjectRequest {
                    bucket,
                    object,
                    ..Default::default()
                };

                let range = Range(Some(start), Some(end));

                client
                    .download_object(&req, &range)
                    .await
                    .map(Bytes::from)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })
        })
    }

    /// Read from cache or fetch new data
    #[tracing::instrument(skip_all)]
    fn get_data(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Check if data is in cache
        if let Some((start, end, ref data)) = self.cache {
            if self.position >= start && self.position < end {
                let offset = (self.position - start) as usize;
                let available = data.len() - offset;
                let to_read = std::cmp::min(available, buf.len());

                buf[..to_read].copy_from_slice(&data[offset..offset + to_read]);
                self.position += to_read as u64;
                self.total_bytes_read += to_read as u64;
                tracing::debug!(
                    "Read {} bytes from cache, total bytes ({})",
                    to_read,
                    self.total_bytes_read
                );

                return Ok(to_read);
            }
        }

        // 8mb chunk size
        const CHUNK_SIZE: u64 = 8 * 1024 * 1024;
        let start = self.position;
        let end = std::cmp::min(start + CHUNK_SIZE, self.total_size);
        tracing::debug!("Fetching range: {}-{}", start, end);

        let data = self.fetch_range(start, end)?;

        let to_read = std::cmp::min(data.len(), buf.len());
        buf[..to_read].copy_from_slice(&data[..to_read]);

        self.cache = Some((start, end, data));
        self.position += to_read as u64;

        self.total_bytes_read += to_read as u64;
        tracing::debug!("Read {} bytes from GCS", to_read);

        Ok(to_read)
    }
}

impl Drop for GcsRangedReader {
    fn drop(&mut self) {
        tracing::debug!(
            "GcsRangedReader dropped. Total bytes read: {}",
            self.total_bytes_read
        );
    }
}

impl Read for GcsRangedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.total_size {
            return Ok(0);
        }

        self.get_data(buf)
    }
}

impl Seek for GcsRangedReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.total_size.saturating_add(offset as u64)
                } else {
                    self.total_size.saturating_sub((-offset) as u64)
                }
            }
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.position.saturating_add(offset as u64)
                } else {
                    self.position.saturating_sub((-offset) as u64)
                }
            }
        };

        if new_pos > self.total_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Cannot seek past end of file (size: {})", self.total_size),
            ));
        }

        self.position = new_pos;
        Ok(self.position)
    }
}
