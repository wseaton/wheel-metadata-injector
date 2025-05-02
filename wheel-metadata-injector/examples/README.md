# Example Usage with setuptools

This directory contains an example of how to use the wheel-metadata-injector as a setuptools plugin.

## Using the Plugin

1. Add `wheel-metadata-injector` to `build-system.requires`, for example, using `setuptools`:

```toml
[build-system]
requires = ["setuptools", "wheel-metadata-injector"]
build-backend = "setuptools.build_meta"
```

Alternatively, it can be manually installed:

```bash
pip install wheel-metadata-injector
```

The package registers itself as a setuptools entry point that overrides the `bdist_wheel` command,
so wheel metadata is injected whenever the package is installed with no extra configuration required.

⚠️ Warning: When using a custom `bdist_wheel` cmdclass inheriting from `setuptools.command.bdist_wheel`,
the custom cmdlass will have to inherit from `InjectMetadataBdistWheel` in order for metadata to be
injected properly:

```python
from wheel_metadata_injector import InjectMetadataBdistWheel

class MyCustomBuildBackend(InjectMetadataBdistWheel):
    ...


setup(
    ...
    cmdclass={
        'bdist_wheel': MyCustomBuildBackend,
    },
    ...
```

## Building a Wheel

After editing the `pyproject.toml` and/or your `setup.py`, build your wheel, e.g. using [`build`](https://pypi.org/project/build/) (`pip install build`)

```bash
python -m build
```

or, using `setup.py bdist_wheel`:

```bash
python setup.py bdist_wheel
```

The plugin will automatically inject the environment metadata into the wheel after it's built.

## Options

You can skip the metadata injection by passing the `--skip-metadata-injection` flag:

```bash
python setup.py bdist_wheel --skip-metadata-injection
```
