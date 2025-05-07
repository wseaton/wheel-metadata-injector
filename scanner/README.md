# Wheel Metadata Scanner

A simple utility to validate metadata from Python wheel packages, with special focus on the build environment metadata that was added by the wheel-metadata-injector.

## Features

- Extract build environment metadata from local wheel files
- Extract build environment metadata from wheels stored in Google Cloud Storage using efficient ranged reads
  - Uses efficient ranged reads from the ZIP file
  - Implements a custom ranged reader for Google Cloud Storage that performs ranged HTTP requests
  - Minimizes network traffic by only fetching the content needed

## TODOs

- Add more validation logic based on the injected metadata format. Currently only environment variable checking is supported. Potentially a DSL or simple query/expression language would be handy?
- Support validations against other standard files in the wheel, which use the RFC882 email header format

## Usage

### Install

From inside this directory: 

```sh
cargo install --locked --path .
```

### Validating a Wheel in a Remote Registry

#### Using CEL

`wheel-metadata-scanner` supports CEL expressions that allow you to craft arbitrarily complex validations:

```bash
YESTERDAY=$(date -u "+%Y-%m-%dT%H:%M:%SZ")

wheel-metadata-scanner 'https://storage.googleapis.com/neuralmagic-public-pypi/dist/xformers-0.0.30+4cf69f09.d20250505-cp312-cp312-linux_x86_64.whl' \
  -c 'metadata.env["CUDA_VERSION"] == "12.8" && metadata.env["TORCH_CUDA_ARCH_LIST"].contains("10.0;12.0") && (timestamp(metadata.build_time) - timestamp("'$YESTERDAY'") < duration("24h"))'
```

#### Using ENV VARs 

```bash
# Extract only the build environment metadata
wheel-metadata-scanner 'https://storage.googleapis.com/neuralmagic-public-pypi/dist/xformers-0.0.30+4cf69f09.d20250505-cp312-cp312-linux_x86_64.whl' -e CUDA_VERSION=12.6
```

here's an example showing a mismatch in expected metadata:

```
2025-05-06T20:18:07.771934Z  INFO extract_from_registry: scanner: Fetching wheel from registry: https://storage.googleapis.com/neuralmagic-public-pypi/dist/xformers-0.0.30+4cf69f09.d20250505-cp312-cp312-linux_x86_64.whl
2025-05-06T20:18:07.913409Z  INFO extract_from_registry: scanner: Wheel size: 54383630 bytes
2025-05-06T20:18:07.913462Z  INFO extract_from_registry: scanner: Creating HTTP ranged reader for URL: https://storage.googleapis.com/neuralmagic-public-pypi/dist/xformers-0.0.30+4cf69f09.d20250505-cp312-cp312-linux_x86_64.whl
2025-05-06T20:18:08.574496Z  INFO extract_from_registry:extract_metadata_from_archive: scanner: Found dist-info directory: xformers-0.0.30+4cf69f09.d20250505.dist-info/
2025-05-06T20:18:08.574547Z  INFO extract_from_registry:extract_metadata_from_archive: scanner: Found WHEEL.metadata (908 bytes)
2025-05-06T20:18:08.574862Z  INFO extract_from_registry:extract_metadata_from_archive: scanner: === Build Environment Metadata ===
# Build environment variables captured during wheel creation
# This file adheres to PEP 658 and contains whitelisted environment variables

build_time = "+002025-05-05T17:30:00.410104985Z"

[git]
url = "https://github.com/facebookresearch/xformers"
commit = "4cf69f0967128217f1798de70b3e4477de138570"

[env]
TORCH_CUDA_ARCH_LIST = "7.5;8.0;8.6;9.0;10.0;12.0+PTX"
CUDA_VERSION = "12.8"
BUILD_TYPE = "RELEASE"
GITHUB_SHA = "661d4dc60faad00bf7e74cef911ed35a25806ad2"
GITHUB_REPOSITORY = "neuralmagic/nm-cicd"
GITHUB_WORKFLOW = "build whl"
GITHUB_JOB = "BUILD"
GITHUB_RUN_ID = "14842309063"
RUNNER_OS = "Linux"
RUNNER_ARCH = "X64"
CMAKE_BUILD_TYPE = "Release"
CFLAGS = "-march=haswell"
CXXFLAGS = "-march=haswell "

[automation]
run_id = "14842309063"
workflow_name = "build whl"
workflow_sha = "661d4dc60faad00bf7e74cef911ed35a25806ad2"
job_name = "BUILD"
runner_name = "k8s-a100-build-12-8-9lb77-runner-z6j4q"

2025-05-06T20:18:08.575916Z  WARN scanner: Environment variable CUDA_VERSION does not match! Expected: 12.46, Found: 12.8
2025-05-06T20:18:08.575946Z ERROR scanner: Found 1 missing or mismatched environment variables!
Error: Found 1 missing or mismatched environment variables!
```

### Validate a Remote Wheel in GCS

Works the exact same as above, just provide `gs://` as the scheme for the URI path:

```bash
# Extract only the build environment metadata
wheel-metadata-scanner gs://your-bucket/path/to/your-package-1.0-py3-none-any.whl
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

The compiled binary will be available at `target/release/wheel-metadata-scanner`.

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