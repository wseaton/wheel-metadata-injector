# Wheel Metadata Injector

A tool that injects build metadata into Python wheel packages according to [PEP 658](https://peps.python.org/pep-0658/).

This allows metadata that the builder would like to get included from the build to be inspected in the final wheel without downloading and installing the entire wheel. A good example and motivation for this is `TORCH_CUDA_ARCH_LIST`, which controls how code is compiled for packages that link to torch. This makes it much easier post-facto to figure out what a wheel was built against, and can potentially serve as inputs for advanced resolvers.

## Installation

```bash
pip install wheel-metadata-injector
```

## Usage

### Command Line Interface

```bash
# Process a wheel file (overwrites the original file)
wheel-metadata-injector path/to/your-package-1.0.0-py3-none-any.whl

# Process a wheel file and save to a new location
wheel-metadata-injector path/to/your-package-1.0.0-py3-none-any.whl -o path/to/output.whl

# Process a wheel file using environment variables from a file
wheel-metadata-injector path/to/your-package-1.0.0-py3-none-any.whl -e path/to/env_vars.txt

# Process a wheel file using inline list of environment variables
wheel-metadata-injector path/to/your-package-1.0.0-py3-none-any.whl -v "PATH,PYTHONPATH,CUDA_VERSION"
```

### Environment Variables Configuration

By default, the tool captures a predefined list of environment variables (see [Whitelisted Environment Variables](#whitelisted-environment-variables) section).

Alternatively, you can provide a file containing a custom list of environment variable names to collect. Example file content:

```
# This is a comment
TORCH_CUDA_ARCH_LIST
CUDA_VERSION
BUILD_TYPE
PATH
PYTHONPATH
# Each line should contain just one variable name
```

### Python API

```python
from wheel_metadata_injector import (
    process_wheel, 
    process_wheel_with_env_file, 
    get_whitelisted_env_vars,
    get_whitelisted_env_vars_with_file
)

# Get environment variables using default whitelist
env_vars = get_whitelisted_env_vars()
print(f"Will inject {len(env_vars)} environment variables")

# Get environment variables using custom list from file
env_vars = get_whitelisted_env_vars_with_file("path/to/env_vars.txt")
print(f"Will inject {len(env_vars)} environment variables from custom list")

# Process a wheel file using system environment variables
output_path = process_wheel("path/to/your-package-1.0.0-py3-none-any.whl")
# or specify output path
output_path = process_wheel("path/to/your-package-1.0.0-py3-none-any.whl", "path/to/output.whl")

# Process a wheel file using custom environment variable list from file
output_path = process_wheel_with_env_file(
    "path/to/your-package-1.0.0-py3-none-any.whl", 
    "path/to/env_vars.txt",
    "path/to/output.whl"  # output path is optional
)

# Process a wheel file using inline list of environment variables
output_path = process_wheel_with_env_vars(
    "path/to/your-package-1.0.0-py3-none-any.whl", 
    "PATH,PYTHONPATH,CUDA_VERSION",
    "path/to/output.whl"  # output path is optional
)
```

### Setuptools Plugin

The package can be used as a setuptools plugin to automatically inject environment metadata when building wheels.

#### Method 1: Import to automatically register the command

```python
# Import the custom wheel command to register it automatically
from wheel_metadata_injector import InjectMetadataBdistWheel

from setuptools import setup, find_packages

setup(
    name="your-package",
    version="0.1.0",
    packages=find_packages(),
    # ... other args
)
```

#### Method 2: Explicitly set the command class

```python
from wheel_metadata_injector import InjectMetadataBdistWheel
from setuptools import setup, find_packages

setup(
    name="your-package",
    version="0.1.0",
    packages=find_packages(),
    cmdclass={
        'bdist_wheel': InjectMetadataBdistWheel,
    },
    # ... other args
)
```

Then build your wheel as usual:

```bash
python setup.py bdist_wheel
```

You can skip the metadata injection by passing the `--skip-metadata-injection` flag:

```bash
python setup.py bdist_wheel --skip-metadata-injection
```

To use environment variables from a file:

```bash
python setup.py bdist_wheel --env-file path/to/env_vars.txt
```

To specify environment variables inline:

```bash
python setup.py bdist_wheel --env-vars "PATH,PYTHONPATH,CUDA_VERSION"
```

See the [examples directory](./examples) for more detailed usage.

## Whitelisted Environment Variables

This tool captures the following environment variables:

- `TORCH_CUDA_ARCH_LIST`
- `CUDA_VERSION`
- `CUDA_HOME`
- `CUDNN_VERSION`
- `BUILD_TYPE`
- `PYTORCH_BUILD_VERSION`
- `PYTORCH_BUILD_NUMBER`
- `CMAKE_ARGS`
- `EXTRA_CAFFE2_CMAKE_FLAGS`

## Development

### Build from source

```bash
# Clone the repository
git clone https://github.com/yourusername/wheel-metadata-injector.git
cd wheel-metadata-injector

# Install development dependencies
pip install maturin

# Build and install in development mode
maturin develop

# Build a wheel
maturin build --release
```

### Running Tests

The project uses pytest for testing and [just](https://github.com/casey/just) as a command runner. 

First, install just and the test dependencies:

```bash
# Install just (on macOS)
brew install just

# On other platforms, see: https://github.com/casey/just#installation

# Install test dependencies
pip install pytest pytest-cov
```

To run the tests using just:

```bash
# Run all tests
just test

# Run tests with coverage report
just coverage

# Run only end-to-end tests
just test-e2e

# See all available commands
just
```

You can also run pytest directly:

```bash
# Build the package first
maturin develop

# Run all tests
pytest
```

See the [tests directory](./tests) for more details on testing.