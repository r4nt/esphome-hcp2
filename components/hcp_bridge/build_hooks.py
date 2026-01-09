import subprocess
import os
import shutil
from esphome.core import CORE
import esphome.codegen as cg
from esphome.codegen import RawStatement
import io

def build_rust_firmware(config):
    # Determine paths
    component_dir = os.path.dirname(__file__)
    base_dir = os.path.dirname(os.path.dirname(component_dir))
    lp_dir = os.path.join(base_dir, "lp-firmware")
    hp_dir = os.path.join(base_dir, "hp-firmware")
    target_dir = os.path.join(base_dir, "target")

    lp_bin_output = CORE.relative_build_path("lp-firmware/hcp2-lp.bin")
    hp_lib_output = CORE.relative_build_path("hp-firmware/libhcp2_hp_lib.a")
    

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

    # Paths to built artifacts (Workspace shares 'target' at root)
    lp_target = "riscv32imac-unknown-none-elf"
    lp_elf_path = os.path.join(target_dir, lp_target, "release", "hcp2-lp")
    hp_lib_path = os.path.join(target_dir, rust_target, "release", "libhcp2_hp_lib.a")
    
    # 3. Process LP Binary
    if is_lp_build:
        print(f"Converting {lp_elf_path} to binary...")
        try:
            os.makedirs(os.path.dirname(lp_bin_output), exist_ok=True)
            subprocess.run([
                "rust-objcopy", 
                "-O", "binary", 
                lp_elf_path, 
                lp_bin_output
            ], check=True)
            
            with open(lp_bin_output, "rb") as f:
                data = f.read()
            with io.StringIO() as f:
                f.write(f"uint8_t lp_firmware_bin[] = {{ \n")
                for i, byte in enumerate(data):
                    f.write(f"0x{byte:02X}, ")
                    if (i + 1) % 16 == 0:
                        f.write("\n")
                f.write("\n};")
                f.write(f"size_t lp_firmware_bin_size = {len(data)};\n")
                cg.add_global(RawStatement(f.getvalue()))
        except Exception as e:
            print(f"Error processing LP artifacts: {e}")
            raise e
    else:  
        # 4. Copy HP Library to Build Dir
        try:
            os.makedirs(os.path.dirname(hp_lib_output), exist_ok=True)
            shutil.copy(hp_lib_path, hp_lib_output)
     
        except Exception as e:
            print(f"Error copying Rust artifacts: {e}")
            raise e
    return is_lp_build
 
# Add to ESPHome build process
def register_build_hooks():
    pass