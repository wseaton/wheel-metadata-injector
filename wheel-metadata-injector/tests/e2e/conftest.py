import sys
import tempfile
import shutil
import subprocess
from pathlib import Path
import pytest


@pytest.fixture
def temp_dir():
    """Create a temporary directory for test files."""
    temp_dir = tempfile.mkdtemp()
    try:
        yield Path(temp_dir)
    finally:
        shutil.rmtree(temp_dir)


@pytest.fixture
def example_wheel(temp_dir):
    """Build an example wheel for testing."""

    example_dir = Path(__file__).parents[2] / "examples" / "example_package"
    setup_py = Path(__file__).parents[2] / "examples" / "setup.py"

    target_dir = temp_dir / "example_package"
    target_dir.mkdir(exist_ok=True)

    shutil.copy(setup_py, temp_dir / "setup.py")
    shutil.copy(example_dir / "__init__.py", target_dir / "__init__.py")

    subprocess.run(
        [sys.executable, "setup.py", "bdist_wheel", "--skip-metadata-injection"],
        cwd=temp_dir,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    dist_dir = temp_dir / "dist"
    wheel_files = list(dist_dir.glob("example*.whl"))
    if not wheel_files:
        pytest.fail("Failed to build example wheel")

    return wheel_files[0]
