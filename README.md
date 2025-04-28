# Wheel Metadata Injector

A tool that injects build environment variables into Python wheel packages according to [PEP 658](https://peps.python.org/pep-0658/).

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
```

### Python API

```python
from wheel_metadata_injector import process_wheel, get_whitelisted_env_vars

# Get current environment variables that will be injected
env_vars = get_whitelisted_env_vars()
print(f"Will inject {len(env_vars)} environment variables")

# Process a wheel file
output_path = process_wheel("path/to/your-package-1.0.0-py3-none-any.whl")
# or specify output path
output_path = process_wheel("path/to/your-package-1.0.0-py3-none-any.whl", "path/to/output.whl")
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