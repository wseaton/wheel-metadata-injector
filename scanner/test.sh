#!/bin/bash
set -e


echo "Building scanner..."
cargo build


uv venv --python=python3.12 venv


echo "Activating virtual environment..."
source "venv/bin/activate"


echo "Ensuring required packages are available..."
uv pip install setuptools wheel

echo "Installing wheel-metadata-injector in development mode..."
pushd ../wheel-metadata-injector > /dev/null
uv pip install -e .
popd > /dev/null


echo "Running tests..."
cargo test -v -- --nocapture

echo "All tests completed successfully!"