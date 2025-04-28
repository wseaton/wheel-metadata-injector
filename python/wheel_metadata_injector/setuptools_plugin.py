import os
from setuptools.command.bdist_wheel import bdist_wheel

from . import process_wheel, get_whitelisted_env_vars

class InjectMetadataBdistWheel(bdist_wheel):
    """A setuptools command for building wheels with injected environment metadata.
    
    This extends the standard bdist_wheel command to automatically inject
    environment metadata after the wheel is built.
    """
    
    user_options = bdist_wheel.user_options + [
        ('skip-metadata-injection', None, 'Skip environment metadata injection'),
    ]
    
    boolean_options = bdist_wheel.boolean_options + ['skip-metadata-injection']
    
    def initialize_options(self):
        super().initialize_options()
        self.skip_metadata_injection = False
    
    def finalize_options(self):
        super().finalize_options()
    
    def run(self):
        # First, build the wheel as normal
        super().run()
        
        if self.skip_metadata_injection:
            print("Skipping environment metadata injection")
            return
        
        # Find the wheel file that was just built
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
        
        env_vars = get_whitelisted_env_vars()
        if not env_vars:
            print("Warning: No whitelisted environment variables found for injection")
        else:
            print(f"Found {len(env_vars)} whitelisted environment variables to inject")
        
        try:
            # Inject metadata in-place
            process_wheel(wheel_path)
            print(f"Successfully injected environment metadata into {wheel_path}")
        except Exception as e:
            print(f"Error injecting environment metadata: {e}")
            raise