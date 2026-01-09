import subprocess
import os
import shutil
from esphome.core import CORE

def build_rust_firmware(config):
    # Determine paths
    component_dir = os.path.dirname(__file__)
    base_dir = os.path.dirname(os.path.dirname(component_dir))
    
    lp_dir = os.path.join(base_dir, "lp-firmware")
    hp_dir = os.path.join(base_dir, "hp-firmware")
    
    lp_bin_output = os.path.join(component_dir, "hcp2-lp.bin")
    lp_header_output = os.path.join(component_dir, "hcp2_lp_bin.h")
    hp_lib_output = os.path.join(component_dir, "libhcp2_hp_lib.a")
    
    target = "riscv32imac-unknown-none-elf"
    
    # 1. Build LP firmware (using its own .cargo/config.toml)
    print("Building HCP2 LP Firmware (Rust)...")
    try:
        subprocess.run([
            "cargo", "build", "--release"
        ], cwd=lp_dir, check=True)
    except Exception as e:
        print(f"Error building LP firmware: {e}")
        return

    # 2. Build HP static lib
    print("Building HCP2 HP Lib (Rust)...")
    try:
        subprocess.run([
            "cargo", "build", "--release", "--target", target
        ], cwd=hp_dir, check=True)
    except Exception as e:
        print(f"Error building HP lib: {e}")
        return

    # Paths to built artifacts (Workspace shares 'target' at root)
    lp_elf_path = os.path.join(base_dir, "target", target, "release", "hcp2-lp")
    hp_lib_path = os.path.join(base_dir, "target", target, "release", "libhcp2_hp_lib.a")
    
    # 3. Convert LP ELF to binary
    print(f"Converting {lp_elf_path} to binary...")
    try:
        subprocess.run([
            "rust-objcopy", 
            "-O", "binary", 
            lp_elf_path, 
            lp_bin_output
        ], check=True)
        
        # Copy HP lib to component dir
        shutil.copy(hp_lib_path, hp_lib_output)
        
        # Generate C header with binary data
        print(f"Generating {lp_header_output}...")
        with open(lp_bin_output, "rb") as f:
            data = f.read()
            
        with open(lp_header_output, "w") as f:
            f.write("#pragma once\n\n")
            f.write(f"// Generated from hcp2-lp.bin, size: {len(data)} bytes\n")
            f.write("#include <stddef.h>\n")
            f.write("#include <stdint.h>\n\n")
            f.write("#ifdef USE_ESP32_VARIANT_ESP32C6\n")
            f.write(f"const uint8_t lp_firmware_bin[] = {{ \n")
            for i, byte in enumerate(data):
                f.write(f"0x{byte:02X}, ")
                if (i + 1) % 16 == 0:
                    f.write("\n")
            f.write("\n\n};")
            f.write(f"const size_t lp_firmware_bin_size = {len(data)};\n")
            f.write("#endif\n")

        # Copy to build directory if it exists
        build_path = CORE.build_path
        if build_path:
            dest_dir = os.path.join(build_path, "src", "esphome", "components", "hcp_bridge")
            os.makedirs(dest_dir, exist_ok=True)
            shutil.copy(lp_bin_output, dest_dir)
            shutil.copy(lp_header_output, dest_dir)
            shutil.copy(hp_lib_output, dest_dir)
            print(f"Copied artifacts to {dest_dir}")

    except Exception as e:
        print(f"Error processing Rust artifacts: {e}")

# Add to ESPHome build process
def register_build_hooks():
    pass
