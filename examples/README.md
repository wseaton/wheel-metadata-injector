# Example Usage with setuptools

This directory contains an example of how to use the wheel-metadata-injector as a setuptools plugin.

## Using the Plugin

1. First, install wheel-metadata-injector:

```bash
pip install wheel-metadata-injector
```

2. In your setup.py, you can use it in two ways:

### Method 1: Automatic detection via entry point

The package registers itself as a setuptools entry point that overrides the `bdist_wheel` command,
so simply importing it is enough:

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

### Method 2: Explicit cmdclass definition

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

## Building a Wheel

After setting up your setup.py, build your wheel as usual:

```bash
python setup.py bdist_wheel
```

The plugin will automatically inject the environment metadata into the wheel after it's built.

## Options

You can skip the metadata injection by passing the `--skip-metadata-injection` flag:

```bash
python setup.py bdist_wheel --skip-metadata-injection
```