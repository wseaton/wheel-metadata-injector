import argparse
import sys
from . import process_wheel, get_whitelisted_env_vars

def main():
    parser = argparse.ArgumentParser(
        description="Inject build environment variables into Python wheel packages"
    )
    parser.add_argument(
        "wheel", help="Path to the wheel file to process"
    )
    parser.add_argument(
        "-o", "--output", help="Output file path (default: overwrites input)"
    )
    
    args = parser.parse_args()
    
    wheel_path = args.wheel
    output_path = args.output
    
    print(f"Processing wheel: {wheel_path}")
    
    env_vars = get_whitelisted_env_vars()
    if not env_vars:
        print("Warning: No whitelisted environment variables found.")
    else:
        print(f"Found {len(env_vars)} whitelisted environment variables")
    
    try:
        output_path = process_wheel(wheel_path, output_path)
        print(f"Successfully processed wheel: {output_path}")
        return 0
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    sys.exit(main())