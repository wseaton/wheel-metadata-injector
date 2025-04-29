# Testing Wheel Metadata Injector

This directory contains tests for the Wheel Metadata Injector project.

## Running Tests

The project uses [just](https://github.com/casey/just) as a command runner. To run tests using just:

```bash
# Run all tests
just test

# Run tests with coverage report
just coverage

# Run tests with HTML coverage report
just coverage-html

# Run only end-to-end tests
just test-e2e
```

Alternatively, you can run pytest directly:

```bash
# Run all tests
pytest

# Run tests with coverage
pytest --cov=wheel_metadata_injector

# Generate a HTML coverage report
pytest --cov=wheel_metadata_injector --cov-report=html
```

## Test Structure

- `e2e/`: End-to-end tests that verify the functionality of the entire package
  - `test_cli.py`: Tests for the command-line interface
  - `test_python_api.py`: Tests for the Python API
  - `test_setuptools_plugin.py`: Tests for the setuptools plugin
  - `conftest.py`: Contains pytest fixtures for testing

## Adding New Tests

When adding new tests, consider whether they are unit tests or end-to-end tests:

- Unit tests should be added to a new `unit/` directory and test individual functions
- End-to-end tests should be added to the `e2e/` directory and test the full functionality