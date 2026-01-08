import subprocess
import os
import shutil
from esphome.core import CORE

def build_rust_firmware(config):
    # Determine paths
    base_dir = os.path.dirname(os.path.dirname(os.path.dirname(__file__)))
    lp_dir = os.path.join(base_dir, "lp-firmware")
    bin_output = os.path.join(base_dir, "components", "hcp_bridge", "hcp2-lp.bin")
    
    # 1. Run cargo build --release
    print("Building HCP2 LP Firmware (Rust)...")
    try:
        subprocess.run(["cargo", "build", "--release"], cwd=lp_dir, check=True)
    except Exception as e:
        print(f"Error building Rust firmware: {e}")
        return

    # 2. Run rust-objcopy
    # We try cargo-objcopy first, then rust-objcopy
    elf_path = os.path.join(base_dir, "target", "riscv32imac-unknown-none-elf", "release", "hcp2-lp")
    print(f"Converting {elf_path} to binary...")
    
    try:
        subprocess.run([
            "rust-objcopy", 
            "-O", "binary", 
            elf_path, 
            bin_output
        ], check=True)
        print(f"Successfully generated {bin_output}")
    except Exception as e:
        print(f"Error during binary conversion: {e}. Ensure 'llvm-tools-preview' is installed.")

# Add to ESPHome build process
def register_build_hooks():
    # This is a bit hacky but works for local component builds
    # We want this to run before the main C++ build starts
    pass
