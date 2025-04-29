import argparse
import sys
from . import (
    process_wheel,
    process_wheel_with_env_file,
    process_wheel_with_env_vars,
    get_whitelisted_env_vars,
    get_whitelisted_env_vars_with_file,
    get_env_vars_from_comma_list,
)


def main():
    parser = argparse.ArgumentParser(
        description="Inject build environment variables into Python wheel packages"
    )
    parser.add_argument("wheel", help="Path to the wheel file to process")
    parser.add_argument(
        "-o", "--output", help="Output file path (default: overwrites input)"
    )
    parser.add_argument(
        "-e",
        "--env-file",
        help="Path to file containing list of environment variables to collect",
    )
    parser.add_argument(
        "-v",
        "--env-vars",
        help="Comma-separated list of environment variables to collect",
    )

    args = parser.parse_args()

    wheel_path = args.wheel
    output_path = args.output
    env_file = args.env_file
    env_vars_list = args.env_vars

    print(f"Processing wheel: {wheel_path}")

    if env_vars_list:
        print(f"Using inline environment variable list: {env_vars_list}")
        env_vars = get_env_vars_from_comma_list(env_vars_list)
    elif env_file:
        print(f"Reading environment variable names from file: {env_file}")
        env_vars = get_whitelisted_env_vars_with_file(env_file)
    else:
        print("Using default whitelisted environment variables")
        env_vars = get_whitelisted_env_vars()

    if not env_vars:
        print("Warning: No environment variables found to inject.")
    else:
        print(f"Found {len(env_vars)} environment variables to inject")
        for name, _ in env_vars:
            print(f"  {name}")

    try:
        if env_vars_list:
            output_path = process_wheel_with_env_vars(
                wheel_path, env_vars_list, output_path
            )
        elif env_file:
            output_path = process_wheel_with_env_file(wheel_path, env_file, output_path)
        else:
            output_path = process_wheel(wheel_path, output_path)

        print(f"Successfully processed wheel: {output_path}")
        return 0
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
