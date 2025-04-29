# Scanner Tests

This directory contains tests for the scanner tool.

## End-to-End Tests

### test_scanner_with_example.rs

This test demonstrates the scanner functionality with an example wheel package:

1. Builds a wheel from the example package in wheel-metadata-injector/examples
2. Injects environment variables into the wheel metadata
3. Runs the scanner tool on the resulting wheel
4. Verifies that the scanner correctly extracts the injected metadata

### test_scanner_all_metadata.rs

This test demonstrates the scanner functionality with the `--all-metadata` flag:

1. Builds a wheel from the example package just like the first test
2. Runs the scanner tool with the `--all-metadata` flag
3. Verifies that the scanner correctly extracts both the injected build metadata and the standard package metadata
4. Checks that the regular wheel METADATA file is also displayed

## Running the Tests

To run these tests, ensure you have:

1. Python 3.8+ installed
2. The wheel-metadata-injector package installed or available in development mode
3. The example package directory accessible

Run all tests with:

```bash
cargo test
```

Or run a specific test with:

```bash
cargo test test_scanner_with_example
cargo test test_scanner_all_metadata
```

You can also use the provided test script which will ensure wheel-metadata-injector is installed:

```bash
./test.sh
```