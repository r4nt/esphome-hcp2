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
    
    # Determine target based on ESP32 variant
    esp32_config = CORE.config.get("esp32", {})
    variant = esp32_config.get("variant", "").lower()
    core_mode = config.get("core", "lp")
    
    # Default to C6 (RISC-V) if unknown
    rust_target = "riscv32imac-unknown-none-elf"
    
    if variant == "esp32c6":
        rust_target = "riscv32imac-unknown-none-elf"
    elif variant == "esp32c3":
        rust_target = "riscv32imc-unknown-none-elf"
    elif variant == "esp32":
        rust_target = "xtensa-esp32-none-elf"
    elif variant == "esp32s2":
        rust_target = "xtensa-esp32s2-none-elf"
    elif variant == "esp32s3":
        rust_target = "xtensa-esp32s3-none-elf"
    elif variant == "esp32h2":
        rust_target = "riscv32imac-unknown-none-elf"
    else:
        print(f"Unknown/Unsupported variant '{variant}'. Defaulting to {rust_target}")

    # Build LP firmware (Only for ESP32-C6 AND when core mode is 'lp')
    is_lp_build = (variant == "esp32c6" and core_mode == "lp")

    if is_lp_build:
        print(f"Building HCP2 LP Firmware (Rust) for {variant}...")
        try:
            # LP always uses C6 target logic
            subprocess.run([
                "cargo", "build", "--release"
            ], cwd=lp_dir, check=True)
        except Exception as e:
            print(f"Error building LP firmware: {e}")
            raise RuntimeError("Rust LP firmware build failed")
    else:
        print(f"Skipping LP Firmware build (Variant='{variant}', Core='{core_mode}')")

    # 2. Build HP static lib (Universal)
    print(f"Building HCP2 HP Lib (Rust) for {rust_target}...")
    try:
        cargo_cmd = ["cargo"]
        extra_args = []
        if rust_target.startswith("xtensa-"):
            cargo_cmd.append("+esp")
            # Custom xtensa targets require building core from source
            extra_args = ["-Z", "build-std=core"]
        
        subprocess.run(
            cargo_cmd + ["build", "--release", "--target", rust_target] + extra_args, 
            cwd=hp_dir, check=True
        )
    except Exception as e:
        print(f"Error building HP lib: {e}")
        print(f"HINT: If building for Xtensa (ESP32/S2/S3), ensure 'espup' toolchain is installed and environment is sourced.")
        raise RuntimeError(f"Rust HP library build failed for {rust_target}")

    # Paths to built artifacts
    # LP is always hardcoded to C6 target in its config
    lp_target = "riscv32imac-unknown-none-elf"
    lp_elf_path = os.path.join(base_dir, "target", lp_target, "release", "hcp2-lp")
    
    # HP uses the detected target
    hp_lib_path = os.path.join(base_dir, "target", rust_target, "release", "libhcp2_hp_lib.a")
    
    # 3. Process LP Binary
    if is_lp_build:
        print(f"Converting {lp_elf_path} to binary...")
        try:
            subprocess.run([
                "rust-objcopy", 
                "-O", "binary", 
                lp_elf_path, 
                lp_bin_output
            ], check=True)
            
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
                f.write(f"const uint8_t lp_firmware_bin[] = {{\n")
                for i, byte in enumerate(data):
                    f.write(f"0x{byte:02X}, ")
                    if (i + 1) % 16 == 0:
                        f.write("\n")
                f.write("\n};\n")
                f.write(f"const size_t lp_firmware_bin_size = {len(data)};\n")
                f.write("#endif\n")
        except Exception as e:
            print(f"Error processing LP artifacts: {e}")
    else:
        # Create dummy header to satisfy component file existence
        # We always overwrite to ensure it's empty in HP mode
        with open(lp_header_output, "w") as f:
            f.write("#pragma once\n// Dummy header for HP-only build\n")

    # 4. Copy Artifacts
    try:
        # Copy HP lib to component dir
        shutil.copy(hp_lib_path, hp_lib_output)

        # Copy to build directory if it exists
        build_path = CORE.build_path
        if build_path:
            dest_dir = os.path.join(build_path, "src", "esphome", "components", "hcp_bridge")
            os.makedirs(dest_dir, exist_ok=True)
            if is_lp_build:
                shutil.copy(lp_bin_output, dest_dir)
            shutil.copy(lp_header_output, dest_dir)
            shutil.copy(hp_lib_output, dest_dir)
            print(f"Copied artifacts to {dest_dir}")

    except Exception as e:
        print(f"Error processing Rust artifacts: {e}")

# Add to ESPHome build process
def register_build_hooks():
    pass
