#![allow(dead_code)]
use anyhow::{Context, Result};
use bytes::Bytes;
use clap::{ArgAction, Parser};

use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;

use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use std::io::{Read, Seek, SeekFrom};
use tokio::task;
use tracing_subscriber::EnvFilter;
use zip::read::ZipArchive;

mod zip_utils;

const BUILD_ENV_FILENAME: &str = "WHEEL.metadata";
const CHUNK_SIZE: u64 = 16 * 1024 * 1024; // 16MB chunk for better caching
const END_OF_ZIP_BUFFER: u64 = 64 * 1024; // 64KB buffer for ZIP central directory

/// Wheel Metadata Scanner - Extract build metadata from Python wheel packages
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to wheel file, HTTP URL, or GCP URL (gs://bucket/path/to/wheel)
    #[clap(value_parser)]
    wheel_path: String,

    /// Environment variable to validate is in the build metadata, a key value pair
    /// in the form of KEY=VALUE
    #[clap(short, long, value_parser=parse_key_val, action = ArgAction::Append)]
    env_var: Vec<(String, String)>,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
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

    let metadata = match args.wheel_path.as_str() {
        path if path.starts_with("gs://") => extract_from_cloud(&args.wheel_path).await,
        path if path.starts_with("http://") || path.starts_with("https://") => {
            extract_from_registry(&args.wheel_path).await
        }
        _ => extract_from_local_file(&args.wheel_path).await,
    };

    let metadata = metadata?;
    let metadata: common::BuildEnvMetadata =
        toml::from_str(&metadata).with_context(|| "Failed to parse metadata as TOML")?;

    // check and make sure the env vars are in the metadata by checking the indexmap in BuildEnvMetadata
    let mut num_missing = 0;

    for (key, value) in args.env_var {
        if let Some(env_value) = metadata.env_vars.get(&key) {
            if env_value != &value {
                tracing::warn!(
                    "Environment variable {} does not match! Expected: {}, Found: {}",
                    key,
                    value,
                    env_value
                );
                num_missing += 1;
            } else {
                tracing::info!("Environment variable {} matches!", key);
            }
        } else {
            tracing::warn!("Environment variable {} not found in metadata!", key);
            num_missing += 1;
        }
    }

    if num_missing > 0 {
        tracing::error!(
            "Found {} missing or mismatched environment variables!",
            num_missing
        );
        return Err(anyhow::anyhow!(
            "Found {} missing or mismatched environment variables!",
            num_missing
        ));
    } else {
        tracing::info!("All environment variables match!");
    }
    tracing::info!("Build environment metadata:");
    println!("{:#?}", metadata);

    Ok(())
}

/// Helper function to extract metadata from a ZIP archive
#[tracing::instrument(skip_all)]
async fn extract_metadata_from_archive<R: Read + Seek>(
    mut archive: ZipArchive<R>,
    _source_desc: &str,
) -> Result<String> {
    // Find the dist-info directory
    let dist_info_dir = zip_utils::find_dist_info_dir(&mut archive)
        .with_context(|| "Failed to find .dist-info directory in wheel")?;

    tracing::info!("Found dist-info directory: {}", dist_info_dir);

    // Extract and output build environment metadata if it exists
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

        return Ok(metadata_content);
    } else {
        tracing::info!("No build environment metadata (WHEEL.metadata) found in wheel!");
        return Err(anyhow::anyhow!(
            "No build environment metadata (WHEEL.metadata) found in wheel!"
        ));
    }
}

/// Extract metadata from a local wheel file
#[tracing::instrument(skip_all)]
async fn extract_from_local_file(wheel_path: &str) -> Result<String> {
    tracing::info!("Reading local wheel file: {}", wheel_path);

    let file = std::fs::File::open(wheel_path)
        .with_context(|| format!("Failed to open wheel file: {}", wheel_path))?;

    let archive = ZipArchive::new(file)
        .with_context(|| format!("Failed to open ZIP archive: {}", wheel_path))?;

    extract_metadata_from_archive(archive, "local file").await
}

/// Do a ranged read from a pypy registry URL, this is downloading just the metadata
/// part of the wheel using a standard HTTP request
#[tracing::instrument(skip_all)]
async fn extract_from_registry(uri: &str) -> Result<String> {
    tracing::info!("Fetching wheel from registry: {}", uri);
    let client = reqwest::Client::new();

    // First, get the total size of the file with a HEAD request
    let head_response = client.head(uri).send().await?;
    let total_size = head_response
        .headers()
        .get("content-length")
        .and_then(|len| len.to_str().ok())
        .and_then(|len| len.parse::<u64>().ok())
        .with_context(|| "Failed to determine file size from HTTP headers")?;

    tracing::info!("Wheel size: {} bytes", total_size);

    let http_reader = HttpRangedReader::new(client, uri.to_string(), total_size).await?;
    let archive = ZipArchive::new(http_reader).context("Failed to open ZIP archive from HTTP")?;

    extract_metadata_from_archive(archive, "HTTP source").await
}

#[tracing::instrument(skip_all)]
async fn extract_from_cloud(uri: &str) -> Result<String> {
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

    let archive = ZipArchive::new(gcs_reader).context("Failed to open ZIP archive from GCS")?;

    extract_metadata_from_archive(archive, "GCS source").await
}

/// Trait for common functionality between different ranged readers
trait RangedReader: Read + Seek {
    /// Get the current position in the file
    fn position(&self) -> u64;

    /// Get the total size of the file
    fn total_size(&self) -> u64;

    /// Get the total bytes read by this reader
    fn total_bytes_read(&self) -> u64;

    /// Fetch a range of bytes from the source
    fn fetch_range(&self, start: u64, end: u64) -> std::io::Result<Bytes>;

    /// Prefetch the ZIP central directory to avoid errors
    fn prefetch_central_directory(&mut self) -> std::io::Result<()> {
        let start = if self.total_size() > END_OF_ZIP_BUFFER {
            self.total_size() - END_OF_ZIP_BUFFER
        } else {
            0
        };

        tracing::debug!(
            "Pre-fetching ZIP end of central directory: {}-{}",
            start,
            self.total_size()
        );

        let data = self.fetch_range(start, self.total_size())?;
        self.update_cache(start, self.total_size(), data);

        tracing::debug!(
            "Prefetched and cached ZIP central directory: {}-{}",
            start,
            self.total_size()
        );

        Ok(())
    }

    /// Read from cache if possible, otherwise fetch new data
    fn get_data_from_cache_or_fetch(
        &mut self,
        buf: &mut [u8],
        cache_hit: bool,
        start: u64,
        end: u64,
        offset: usize,
    ) -> std::io::Result<usize> {
        if cache_hit {
            return self.read_from_cache(buf, offset);
        }

        // Handle reading data
        tracing::debug!("Fetching range: {}-{}", start, end);
        let data = self.fetch_range(start, end)?;

        // Calculate position offset and available data
        let position_offset = (self.position() - start) as usize;
        let to_read = std::cmp::min(data.len().saturating_sub(position_offset), buf.len());

        if position_offset < data.len() {
            buf[..to_read].copy_from_slice(&data[position_offset..position_offset + to_read]);
        } else {
            // Handle edge case where position offset exceeds data length
            return Ok(0);
        }

        // Store in cache and update position
        self.update_cache(start, end, data);
        self.update_position_and_bytes_read(to_read as u64);

        Ok(to_read)
    }

    /// Update the cache with new data
    fn update_cache(&mut self, start: u64, end: u64, data: Bytes);

    /// Read data from the cache
    fn read_from_cache(&mut self, buf: &mut [u8], offset: usize) -> std::io::Result<usize>;

    /// Update position and bytes read counter
    fn update_position_and_bytes_read(&mut self, bytes_read: u64);

    /// Check if the requested range is in cache
    fn is_in_cache(&self, position: u64) -> Option<(u64, u64, usize)>;

    /// Get name for logging
    fn reader_name(&self) -> &str;

    /// Ensure end of central directory is in cache
    fn ensure_end_of_central_directory(&mut self) -> std::io::Result<()> {
        // Special handling for ZIP file structure
        // Only do this if we're reading near the end of the file
        if self.position() > self.total_size() - 4096 && self.position() < self.total_size() {
            let start = if self.total_size() > END_OF_ZIP_BUFFER {
                self.total_size() - END_OF_ZIP_BUFFER
            } else {
                0
            };
            let end = self.total_size();

            // Check if we've already cached this range or need to update
            if let Some((cached_start, cached_end, _)) = self.is_in_cache(self.position()) {
                if cached_start != start || cached_end != end {
                    tracing::debug!("Fetching ZIP footer: {}-{}", start, end);
                    let data = self.fetch_range(start, end)?;
                    self.update_cache(start, end, data);
                }
            } else {
                tracing::debug!("Fetching ZIP footer: {}-{}", start, end);
                let data = self.fetch_range(start, end)?;
                self.update_cache(start, end, data);
            }
        }

        Ok(())
    }
}

// Implement common Seek functionality
fn common_seek(position: u64, total_size: u64, pos: SeekFrom) -> std::io::Result<u64> {
    let new_pos = match pos {
        SeekFrom::Start(offset) => offset,
        SeekFrom::End(offset) => {
            if offset >= 0 {
                total_size.saturating_add(offset as u64)
            } else {
                total_size.saturating_sub((-offset) as u64)
            }
        }
        SeekFrom::Current(offset) => {
            if offset >= 0 {
                position.saturating_add(offset as u64)
            } else {
                position.saturating_sub((-offset) as u64)
            }
        }
    };

    if new_pos > total_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Cannot seek past end of file (size: {})", total_size),
        ));
    }

    Ok(new_pos)
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

        let mut reader = Self {
            client,
            bucket,
            object,
            position: 0,
            total_size,
            cache: None,
            total_bytes_read: 0,
        };

        // Pre-fetch the central directory for ZIP reading
        if total_size > 0 {
            reader.prefetch_central_directory()?;
        }

        Ok(reader)
    }
}

impl RangedReader for GcsRangedReader {
    fn position(&self) -> u64 {
        self.position
    }

    fn total_size(&self) -> u64 {
        self.total_size
    }

    fn total_bytes_read(&self) -> u64 {
        self.total_bytes_read
    }

    fn fetch_range(&self, start: u64, end: u64) -> std::io::Result<Bytes> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let object = self.object.clone();

        task::block_in_place(|| {
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

    fn update_cache(&mut self, start: u64, end: u64, data: Bytes) {
        self.cache = Some((start, end, data));
    }

    fn read_from_cache(&mut self, buf: &mut [u8], offset: usize) -> std::io::Result<usize> {
        if let Some((_, _, ref data)) = self.cache {
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

        Ok(0)
    }

    fn update_position_and_bytes_read(&mut self, bytes_read: u64) {
        self.position += bytes_read;
        self.total_bytes_read += bytes_read;
        tracing::debug!("Read {} bytes from {}", bytes_read, self.reader_name());
    }

    fn is_in_cache(&self, position: u64) -> Option<(u64, u64, usize)> {
        if let Some((start, end, ref _data)) = self.cache {
            if position >= start && position < end {
                return Some((start, end, (position - start) as usize));
            }
        }
        None
    }

    fn reader_name(&self) -> &str {
        "GCS"
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

        self.ensure_end_of_central_directory()?;

        if let Some((_start, _end, offset)) = self.is_in_cache(self.position) {
            return self.read_from_cache(buf, offset);
        }

        let start = self.position;
        let end = std::cmp::min(start + CHUNK_SIZE, self.total_size);

        self.get_data_from_cache_or_fetch(buf, false, start, end, 0)
    }
}

impl Seek for GcsRangedReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = common_seek(self.position, self.total_size, pos)?;
        self.position = new_pos;
        Ok(self.position)
    }
}

/// A struct that implements Read and Seek for HTTP resources with ranged reads
struct HttpRangedReader {
    client: reqwest::Client,
    url: String,
    position: u64,
    total_size: u64,
    cache: Option<(u64, u64, Bytes)>, // (start, end, data)
    total_bytes_read: u64,
}

impl HttpRangedReader {
    /// Create a new HttpRangedReader
    async fn new(client: reqwest::Client, url: String, total_size: u64) -> Result<Self> {
        tracing::info!("Creating HTTP ranged reader for URL: {}", url);

        let mut reader = Self {
            client,
            url,
            position: 0,
            total_size,
            cache: None,
            total_bytes_read: 0,
        };

        if total_size > 0 {
            reader.prefetch_central_directory()?;
        }

        Ok(reader)
    }
}

impl RangedReader for HttpRangedReader {
    fn position(&self) -> u64 {
        self.position
    }

    fn total_size(&self) -> u64 {
        self.total_size
    }

    fn total_bytes_read(&self) -> u64 {
        self.total_bytes_read
    }

    fn fetch_range(&self, start: u64, end: u64) -> std::io::Result<Bytes> {
        let client = self.client.clone();
        let url = self.url.clone();

        task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            rt.block_on(async move {
                let range_header = format!("bytes={}-{}", start, end - 1);

                let mut headers = HeaderMap::new();
                headers.insert(RANGE, HeaderValue::from_str(&range_header).unwrap());

                // Make the range request
                let response = client
                    .get(&url)
                    .headers(headers)
                    .send()
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

                if !response.status().is_success() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("HTTP request failed: {}", response.status()),
                    ));
                }

                response
                    .bytes()
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })
        })
    }

    fn update_cache(&mut self, start: u64, end: u64, data: Bytes) {
        self.cache = Some((start, end, data));
    }

    fn read_from_cache(&mut self, buf: &mut [u8], offset: usize) -> std::io::Result<usize> {
        if let Some((_, _, ref data)) = self.cache {
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

        Ok(0)
    }

    fn update_position_and_bytes_read(&mut self, bytes_read: u64) {
        self.position += bytes_read;
        self.total_bytes_read += bytes_read;
        tracing::debug!("Read {} bytes from {}", bytes_read, self.reader_name());
    }

    fn is_in_cache(&self, position: u64) -> Option<(u64, u64, usize)> {
        if let Some((start, end, ref _data)) = self.cache {
            if position >= start && position < end {
                return Some((start, end, (position - start) as usize));
            }
        }
        None
    }

    fn reader_name(&self) -> &str {
        "HTTP"
    }
}

impl Drop for HttpRangedReader {
    fn drop(&mut self) {
        tracing::debug!(
            "HttpRangedReader dropped. Total bytes read: {}",
            self.total_bytes_read
        );
    }
}

impl Read for HttpRangedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.total_size {
            return Ok(0);
        }

        // First, ensure we have the ZIP central directory if needed
        self.ensure_end_of_central_directory()?;

        if let Some((_start, _end, offset)) = self.is_in_cache(self.position) {
            return self.read_from_cache(buf, offset);
        }

        // If not in cache, determine range to fetch
        // Align the start position to a 4K boundary to improve cache efficiency
        let aligned_start = self.position & !(4096 - 1);
        let start = aligned_start;
        let end = std::cmp::min(start + CHUNK_SIZE, self.total_size);

        self.get_data_from_cache_or_fetch(buf, false, start, end, 0)
    }
}

impl Seek for HttpRangedReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = common_seek(self.position, self.total_size, pos)?;
        self.position = new_pos;
        Ok(self.position)
    }
}
