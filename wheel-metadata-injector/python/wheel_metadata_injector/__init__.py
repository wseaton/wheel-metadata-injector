from ._wheel_metadata_injector import (
    process_wheel,
    process_wheel_with_env_file,
    process_wheel_with_env_vars,
    get_whitelisted_env_vars,
    get_whitelisted_env_vars_with_file,
    get_env_vars_from_comma_list,
)

__all__ = [
    "process_wheel",
    "process_wheel_with_env_file",
    "process_wheel_with_env_vars",
    "get_whitelisted_env_vars",
    "get_whitelisted_env_vars_with_file",
    "get_env_vars_from_comma_list",
]

try:
    from .setuptools_plugin import InjectMetadataBdistWheel  # noqa: F401

    __all__.append("InjectMetadataBdistWheel")
except ImportError:
    # If setuptools is not available, we can still import the other functions (like what the CLI requires),
    # but we cannot define the InjectMetadataBdistWheel class.
    pass
