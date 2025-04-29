# Wheel Metadata Scanner

A simple utility to extract metadata from Python wheel packages, with special focus on the build environment metadata that was added by the wheel-metadata-injector.

## Features

- Extract build environment metadata from local wheel files
- Extract build environment metadata from wheels stored in Google Cloud Storage using efficient ranged reads
  - Uses efficient ranged reads from the ZIP file
  - Implements a custom ranged reader for Google Cloud Storage that performs ranged HTTP requests
  - Minimizes network traffic by only fetching the content needed
- Optionally extract the full package metadata

## Usage

### Scanning a Local Wheel

```bash
# Extract only the build environment metadata
cargo run -- /path/to/your-package-1.0-py3-none-any.whl

# Extract all metadata (build environment + package metadata)
cargo run -- /path/to/your-package-1.0-py3-none-any.whl --all-metadata
```

### Scanning a Remote Wheel in GCS

```bash
# Extract only the build environment metadata
cargo run -- gs://your-bucket/path/to/your-package-1.0-py3-none-any.whl

# Extract all metadata (build environment + package metadata)
cargo run -- gs://your-bucket/path/to/your-package-1.0-py3-none-any.whl --all-metadata
```

## GCS Authentication

For GCS access, the scanner uses the standard Google Cloud authentication methods:

1. Application Default Credentials (ADC) if running in a GCP environment
2. User credentials from gcloud CLI if running locally
3. Service account key file specified via the `GOOGLE_APPLICATION_CREDENTIALS` environment variable

## Build

```bash
cargo build --release
```

The compiled binary will be available at `target/release/scanner`.

## Testing

The scanner includes tests that demonstrate usage with wheel files built by the wheel-metadata-injector:

```bash
# Run all tests
cargo test

# Or use the provided test script which ensures wheel-metadata-injector is installed
./test.sh
```

Tests include:
- Basic functionality test with wheel-metadata-injector example package
- Testing the `--all-metadata` flag to extract complete package metadata

See the [tests directory](./tests) for more information on the test scenarios and how to run them.