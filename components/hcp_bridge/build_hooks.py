import subprocess
import os
import shutil
from esphome.core import CORE
import esphome.codegen as cg
from esphome.codegen import RawStatement
import io

def is_lp_mode(config):
    esp32_config = CORE.config.get("esp32", {})
    variant = esp32_config.get("variant", "").lower()
    core_mode = config.get("core", "lp")
    return variant == "esp32c6" and core_mode == "lp"

def _get_paths():
    component_dir = os.path.dirname(__file__)
    base_dir = os.path.dirname(os.path.dirname(component_dir))
    lp_dir = os.path.join(base_dir, "lp-firmware")
    hp_dir = os.path.join(base_dir, "hp-firmware")
    target_dir = os.path.join(base_dir, "target")
    return base_dir, lp_dir, hp_dir, target_dir

def _get_rust_target():
    esp32_config = CORE.config.get("esp32", {})
    variant = esp32_config.get("variant", "").lower()
    
    if variant == "esp32c6":
        return "riscv32imac-unknown-none-elf"
    elif variant == "esp32c3":
        return "riscv32imc-unknown-none-elf"
    elif variant == "esp32":
        return "xtensa-esp32-none-elf"
    elif variant == "esp32s2":
        return "xtensa-esp32s2-none-elf"
    elif variant == "esp32s3":
        return "xtensa-esp32s3-none-elf"
    elif variant == "esp32h2":
        return "riscv32imac-unknown-none-elf"
    else:
        print(f"Unknown/Unsupported variant '{variant}'. Defaulting to riscv32imac-unknown-none-elf")
        return "riscv32imac-unknown-none-elf"

def build_lp_firmware(config):
    base_dir, lp_dir, _, target_dir = _get_paths()
    lp_bin_output = CORE.relative_build_path("lp-firmware/hcp2-lp.bin")
    
    # LP always uses C6 target logic
    print(f"Building HCP2 LP Firmware (Rust)...")
    try:
        subprocess.run([
            "cargo", "build", "--release"
        ], cwd=lp_dir, check=True)
    except Exception as e:
        print(f"Error building LP firmware: {e}")
        raise RuntimeError("Rust LP firmware build failed")

    # Paths to built artifacts
    lp_target = "riscv32imac-unknown-none-elf"
    lp_elf_path = os.path.join(target_dir, lp_target, "release", "hcp2-lp")
    
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

def build_hp_firmware(config):
    base_dir, _, hp_dir, target_dir = _get_paths()
    hp_lib_output = CORE.relative_build_path("hp-firmware/libhcp2_hp_lib.a")
    rust_target = _get_rust_target()

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

    hp_lib_path = os.path.join(target_dir, rust_target, "release", "libhcp2_hp_lib.a")
    
    try:
        os.makedirs(os.path.dirname(hp_lib_output), exist_ok=True)
        shutil.copy(hp_lib_path, hp_lib_output)

    except Exception as e:
        print(f"Error copying Rust artifacts: {e}")
        raise e