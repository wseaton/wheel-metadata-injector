import os
import zipfile
from wheel_metadata_injector import (
    process_wheel,
    process_wheel_with_env_file,
    process_wheel_with_env_vars,
    get_whitelisted_env_vars_with_file,
    get_env_vars_from_comma_list,
)


def test_process_wheel(example_wheel, temp_dir):
    """Test the process_wheel function."""
    output_wheel = temp_dir / "output.whl"

    result_path = process_wheel(str(example_wheel), str(output_wheel))

    assert result_path == str(output_wheel)
    assert output_wheel.exists()

    with zipfile.ZipFile(output_wheel) as wheel_zip:
        assert any(
            name.endswith(".dist-info/WHEEL.metadata") for name in wheel_zip.namelist()
        )


def test_process_wheel_with_env_vars(example_wheel, temp_dir):
    """Test processing with specific environment variables."""
    output_wheel = temp_dir / "output.whl"

    os.environ["TEST_ENV_VAR"] = "test_value"
    os.environ["ANOTHER_TEST_VAR"] = "another_value"

    result_path = process_wheel_with_env_vars(
        str(example_wheel), "TEST_ENV_VAR,ANOTHER_TEST_VAR", str(output_wheel)
    )

    assert result_path == str(output_wheel)
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


def test_process_wheel_with_env_file(example_wheel, temp_dir):
    """Test processing with environment variables from file."""
    output_wheel = temp_dir / "output.whl"

    os.environ["TEST_ENV_VAR"] = "test_value"
    os.environ["ANOTHER_TEST_VAR"] = "another_value"

    env_file = temp_dir / "env_vars.txt"
    with open(env_file, "w") as f:
        f.write("TEST_ENV_VAR\nANOTHER_TEST_VAR")

    result_path = process_wheel_with_env_file(
        str(example_wheel), str(env_file), str(output_wheel)
    )

    assert result_path == str(output_wheel)
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


def test_get_env_vars_functions(temp_dir):
    """Test the functions for getting environment variables."""

    os.environ["TEST_ENV_VAR"] = "test_value"
    os.environ["ANOTHER_TEST_VAR"] = "another_value"

    env_vars = get_env_vars_from_comma_list("TEST_ENV_VAR,ANOTHER_TEST_VAR")
    assert len(env_vars) == 2
    assert ("TEST_ENV_VAR", "test_value") in env_vars
    assert ("ANOTHER_TEST_VAR", "another_value") in env_vars

    env_file = temp_dir / "env_vars.txt"
    with open(env_file, "w") as f:
        f.write("TEST_ENV_VAR\nANOTHER_TEST_VAR")

    env_vars = get_whitelisted_env_vars_with_file(str(env_file))
    assert len(env_vars) == 2
    assert ("TEST_ENV_VAR", "test_value") in env_vars
    assert ("ANOTHER_TEST_VAR", "another_value") in env_vars
