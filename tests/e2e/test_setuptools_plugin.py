import os
import sys
import subprocess
import zipfile
from pathlib import Path


def test_setuptools_plugin(temp_dir):
    """Test the setuptools plugin functionality."""

    example_dir = Path(__file__).parents[2] / "examples" / "example_package"
    setup_py = Path(__file__).parents[2] / "examples" / "setup.py"

    target_dir = temp_dir / "example_package"
    target_dir.mkdir(exist_ok=True)

    with open(setup_py, "r") as f:
        setup_content = f.read()

    with open(target_dir.parent / "setup.py", "w") as f:
        f.write(setup_content)

    with open(example_dir / "__init__.py", "r") as f:
        init_content = f.read()

    with open(target_dir / "__init__.py", "w") as f:
        f.write(init_content)

    os.environ["TEST_ENV_VAR"] = "test_value"

    result = subprocess.run(
        [sys.executable, "setup.py", "bdist_wheel", "--env-vars", "TEST_ENV_VAR"],
        cwd=temp_dir,
        check=True,
        capture_output=True,
        text=True,
    )

    dist_dir = temp_dir / "dist"
    wheel_files = list(dist_dir.glob("*.whl"))
    assert wheel_files, "Failed to build example wheel"

    wheel_file = wheel_files[0]

    with zipfile.ZipFile(wheel_file) as wheel_zip:
        metadata_files = [
            name
            for name in wheel_zip.namelist()
            if name.endswith(".dist-info/WHEEL.metadata")
        ]
        assert metadata_files

        metadata_content = wheel_zip.read(metadata_files[0]).decode("utf-8")
        assert "TEST_ENV_VAR" in metadata_content
        assert "test_value" in metadata_content


def test_setuptools_plugin_with_env_file(temp_dir):
    """Test the setuptools plugin with environment variables from file."""

    example_dir = Path(__file__).parents[2] / "examples" / "example_package"
    setup_py = Path(__file__).parents[2] / "examples" / "setup.py"

    target_dir = temp_dir / "example_package"
    target_dir.mkdir(exist_ok=True)

    with open(setup_py, "r") as f:
        setup_content = f.read()

    with open(target_dir.parent / "setup.py", "w") as f:
        f.write(setup_content)

    with open(example_dir / "__init__.py", "r") as f:
        init_content = f.read()

    with open(target_dir / "__init__.py", "w") as f:
        f.write(init_content)

    os.environ["TEST_ENV_VAR"] = "test_value"
    os.environ["ANOTHER_TEST_VAR"] = "another_value"

    env_file = temp_dir / "env_vars.txt"
    with open(env_file, "w") as f:
        f.write("TEST_ENV_VAR\nANOTHER_TEST_VAR")

    result = subprocess.run(
        [sys.executable, "setup.py", "bdist_wheel", "--env-file", str(env_file)],
        cwd=temp_dir,
        check=True,
        capture_output=True,
        text=True,
    )

    dist_dir = temp_dir / "dist"
    wheel_files = list(dist_dir.glob("*.whl"))
    assert wheel_files, "Failed to build example wheel"

    wheel_file = wheel_files[0]

    with zipfile.ZipFile(wheel_file) as wheel_zip:
        metadata_files = [
            name
            for name in wheel_zip.namelist()
            if name.endswith(".dist-info/WHEEL.metadata")
        ]
        assert metadata_files

        metadata_content = wheel_zip.read(metadata_files[0]).decode("utf-8")
        assert "TEST_ENV_VAR" in metadata_content
        assert "test_value" in metadata_content
        assert "ANOTHER_TEST_VAR" in metadata_content
        assert "another_value" in metadata_content
