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
        
        # Generate C header with binary data
        header_output = os.path.join(base_dir, "components", "hcp_bridge", "hcp2_lp_bin.h")
        print(f"Generating {header_output}...")
        
        with open(bin_output, "rb") as f:
            data = f.read()
            
        with open(header_output, "w") as f:
            f.write("#pragma once\n\n")
            f.write(f"// Generated from hcp2-lp.bin, size: {len(data)} bytes\n")
            f.write("#include <stddef.h>\n")
            f.write("#include <stdint.h>\n\n")
            f.write("#ifdef USE_ESP32_VARIANT_ESP32C6\n")
            f.write(f"const uint8_t lp_firmware_bin[] = {{\n")
            
            for i, byte in enumerate(data):
                f.write(f"0x{byte:02X}, ")
                if (i + 1) % 16 == 0:
                    f.write("\n")
            
            f.write("\n};")
            f.write(f"const size_t lp_firmware_bin_size = {len(data)};\n")
            f.write("#endif\n")

        # Copy to build directory if it exists
        build_path = CORE.build_path
        if build_path:
            dest_dir = os.path.join(build_path, "src", "esphome", "components", "hcp_bridge")
            if os.path.exists(dest_dir):
                shutil.copy(bin_output, dest_dir)
                shutil.copy(header_output, dest_dir)
                print(f"Copied artifacts to {dest_dir}")

    except Exception as e:
        print(f"Error during binary conversion: {e}. Ensure 'llvm-tools-preview' is installed.")

# Add to ESPHome build process
def register_build_hooks():
    # This is a bit hacky but works for local component builds
    # We want this to run before the main C++ build starts
    pass