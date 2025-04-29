from setuptools import setup, find_packages


from wheel_metadata_injector import InjectMetadataBdistWheel

setup(
    name="example-package",
    version="0.1.0",
    packages=find_packages(),
    python_requires=">=3.8",
    cmdclass={
        "bdist_wheel": InjectMetadataBdistWheel,
    },
)
