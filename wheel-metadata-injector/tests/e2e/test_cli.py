import os
import subprocess
import zipfile
import pytest


@pytest.mark.skip("")
def test_cli_basic(example_wheel, temp_dir):
    """Test the basic CLI functionality."""
    output_wheel = temp_dir / "output.whl"

    result = subprocess.run(
        ["wheel-metadata-injector", str(example_wheel), "-o", str(output_wheel)],
        check=True,
        capture_output=True,
        text=True,
    )

    assert "Successfully processed wheel" in result.stdout
    assert output_wheel.exists()

    with zipfile.ZipFile(output_wheel) as wheel_zip:
        assert any(
            name.endswith(".dist-info/WHEEL.metadata") for name in wheel_zip.namelist()
        )


@pytest.mark.skip("")
def test_cli_env_vars(example_wheel, temp_dir):
    """Test the CLI with custom environment variables."""
    output_wheel = temp_dir / "output.whl"

    os.environ["TEST_ENV_VAR"] = "test_value"

    result = subprocess.run(
        [
            "wheel-metadata-injector",
            str(example_wheel),
            "-o",
            str(output_wheel),
            "-v",
            "TEST_ENV_VAR",
        ],
        check=True,
        capture_output=True,
        text=True,
    )

    assert "Successfully processed wheel" in result.stdout
    assert output_wheel.exists()

    with zipfile.ZipFile(output_wheel) as wheel_zip:
        metadata_files = [
            name
            for name in wheel_zip.namelist()
            if name.endswith(".dist-info/WHEEL.metadata")
        ]
        assert metadata_files

        metadata_content = wheel_zip.read(metadata_files[0]).decode("utf-8")
        assert "TEST_ENV_VAR" in metadata_content
        assert "test_value" in metadata_content


@pytest.mark.skip("")
def test_cli_env_file(example_wheel, temp_dir):
    """Test the CLI with environment variables from file."""
    output_wheel = temp_dir / "output.whl"

    os.environ["TEST_ENV_VAR"] = "test_value"
    os.environ["ANOTHER_TEST_VAR"] = "another_value"

    env_file = temp_dir / "env_vars.txt"
    with open(env_file, "w") as f:
        f.write("TEST_ENV_VAR\nANOTHER_TEST_VAR")

    result = subprocess.run(
        [
            "wheel-metadata-injector",
            str(example_wheel),
            "-o",
            str(output_wheel),
            "-e",
            str(env_file),
        ],
        check=True,
        capture_output=True,
        text=True,
    )

    assert "Successfully processed wheel" in result.stdout
    assert output_wheel.exists()

    with zipfile.ZipFile(output_wheel) as wheel_zip:
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

        print(metadata_content)
