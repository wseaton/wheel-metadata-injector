mod core {
    pub use wheel_metadata_injector::collect_whitelisted_env_vars;
    pub use wheel_metadata_injector::unpack_wheel;
    pub use wheel_metadata_injector::create_build_env_file;
    pub use wheel_metadata_injector::update_record_file;
    pub use wheel_metadata_injector::repack_wheel;
    pub use wheel_metadata_injector::WheelInfo;
    pub use wheel_metadata_injector::ENV_WHITELIST;
    pub use wheel_metadata_injector::BUILD_ENV_FILENAME;
}

use std::io;
use clap::{Arg, Command};

fn main() -> io::Result<()> {
    let matches = Command::new("Wheel Metadata Injector")
        .version("1.0")
        .author("Your Name")
        .about("Injects build environment variables into Python wheel packages")
        .arg(
            Arg::new("wheel")
                .help("Path to the wheel file to process")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file path (default: overwrites input)"),
        )
        .get_matches();

    let wheel_path = matches.get_one::<String>("wheel").unwrap();
    let output_path = matches.get_one::<String>("output").unwrap_or(wheel_path);

    println!("Processing wheel: {}", wheel_path);

    let env_vars = core::collect_whitelisted_env_vars();
    if env_vars.is_empty() {
        println!("Warning: No whitelisted environment variables found.");
        println!("Looked for: {:?}", core::ENV_WHITELIST);
    } else {
        println!("Found {} whitelisted environment variables", env_vars.len());
    }

    let temp_dir = tempfile::tempdir()?;
    let temp_dir_path = temp_dir.path();

    let wheel_info = core::unpack_wheel(wheel_path, temp_dir_path)?;
    let build_env_path = temp_dir_path
        .join(&wheel_info.dist_info_dir)
        .join(core::BUILD_ENV_FILENAME);
    core::create_build_env_file(&build_env_path, &env_vars)?;
    core::update_record_file(temp_dir_path, &wheel_info, &build_env_path)?;
    core::repack_wheel(temp_dir_path, output_path)?;

    println!("Successfully processed wheel: {}", output_path);
    Ok(())
}

