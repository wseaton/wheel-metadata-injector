mod core {
    pub use wheel_metadata_injector::collect_whitelisted_env_vars;
    pub use wheel_metadata_injector::collect_whitelisted_env_vars_with_file;
    pub use wheel_metadata_injector::collect_env_vars_from_comma_list;
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
        .arg(
            Arg::new("env_file")
                .short('e')
                .long("env-file")
                .value_name("FILE")
                .help("Path to file containing list of environment variables to collect"),
        )
        .arg(
            Arg::new("env_vars")
                .short('v')
                .long("env-vars")
                .value_name("VARS")
                .help("Comma-separated list of environment variables to collect"),
        )
        .get_matches();

    let wheel_path = matches.get_one::<String>("wheel").unwrap();
    let output_path = matches.get_one::<String>("output").unwrap_or(wheel_path);
    let env_file = matches.get_one::<String>("env_file");
    let env_vars_list = matches.get_one::<String>("env_vars");

    println!("Processing wheel: {}", wheel_path);

    let env_vars = if let Some(vars) = env_vars_list {
        println!("Using inline environment variable list: {}", vars);
        core::collect_env_vars_from_comma_list(vars)
    } else if let Some(file_path) = env_file {
        println!("Reading environment variable names from file: {}", file_path);
        core::collect_whitelisted_env_vars_with_file(Some(file_path))
    } else {
        println!("Using default whitelisted environment variables");
        core::collect_whitelisted_env_vars()
    };

    if env_vars.is_empty() {
        println!("Warning: No environment variables found to inject.");
        if env_file.is_none() {
            println!("Looked for default variables: {:?}", core::ENV_WHITELIST);
        }
    } else {
        println!("Found {} environment variables to inject", env_vars.len());
        for (name, value) in &env_vars {
            println!("  {}", name);
        }
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

