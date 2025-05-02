import os
from setuptools.command.bdist_wheel import bdist_wheel

from . import (
    process_wheel,
    process_wheel_with_env_file,
    get_whitelisted_env_vars,
    get_whitelisted_env_vars_with_file,
)


class InjectMetadataBdistWheel(bdist_wheel):
    """A setuptools command for building wheels with injected environment metadata.

    This extends the standard bdist_wheel command to automatically inject
    environment metadata after the wheel is built.
    """

    user_options = bdist_wheel.user_options + [
        ("skip-metadata-injection", None, "Skip environment metadata injection"),
        (
            "env-file=",
            "e",
            "Path to file containing list of environment variables to collect",
        ),
        ("env-vars=", None, "Comma-separated list of environment variables to collect"),
    ]

    boolean_options = bdist_wheel.boolean_options + ["skip-metadata-injection"]

    def initialize_options(self):
        super().initialize_options()
        self.skip_metadata_injection = False
        self.env_file = None
        self.env_vars = None

    def finalize_options(self):
        super().finalize_options()

    def run(self):
        super().run()

        if self.skip_metadata_injection:
            print("Skipping environment metadata injection")
            return

        wheel_dir = self.dist_dir
        wheel_name = self.wheel_dist_name
        wheel_pattern = f"{wheel_name}*.whl"

        import glob

        wheels = glob.glob(os.path.join(wheel_dir, wheel_pattern))

        if not wheels:
            print(f"Warning: No wheels found matching {wheel_pattern} in {wheel_dir}")
            return

        wheel_path = wheels[0]

        print(f"Injecting environment metadata into {wheel_path}")

        temp_env_file = None

        try:
            if self.env_vars:
                print(f"Using inline environment variable list: {self.env_vars}")
                import tempfile

                fd, temp_path = tempfile.mkstemp(
                    prefix="wheel_metadata_", suffix=".txt"
                )
                temp_env_file = temp_path

                with os.fdopen(fd, "w") as f:
                    for var in self.env_vars.split(","):
                        var = var.strip()
                        if var:
                            f.write(f"{var}\n")

                env_vars = get_whitelisted_env_vars_with_file(temp_path)
            elif self.env_file:
                print(f"Reading environment variable names from file: {self.env_file}")
                env_vars = get_whitelisted_env_vars_with_file(self.env_file)
            else:
                print("Using default whitelisted environment variables")
                env_vars = get_whitelisted_env_vars()

            if not env_vars:
                print("Warning: No environment variables found to inject")
            else:
                print(f"Found {len(env_vars)} environment variables to inject")
                for name, _ in env_vars:
                    print(f"  {name}")

            try:
                if self.env_vars and temp_env_file:
                    process_wheel_with_env_file(wheel_path, temp_env_file, None)
                elif self.env_file:
                    process_wheel_with_env_file(wheel_path, self.env_file, None)
                else:
                    process_wheel(wheel_path, None)
                print(f"Successfully injected environment metadata into {wheel_path}")
            except Exception as e:
                print(f"Error injecting environment metadata: {e}")
                raise
        finally:
            if temp_env_file and os.path.exists(temp_env_file):
                try:
                    os.unlink(temp_env_file)
                except Exception as e:
                    print(
                        f"Warning: Failed to remove temporary file {temp_env_file}: {e}"
                    )
