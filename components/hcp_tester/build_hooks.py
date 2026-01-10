import subprocess
import os
import shutil
from esphome.core import CORE

def _get_paths():
    component_dir = os.path.dirname(__file__)
    base_dir = os.path.dirname(os.path.dirname(component_dir))
    tester_dir = os.path.join(base_dir, "tester-firmware")
    target_dir = os.path.join(base_dir, "target")
    return base_dir, tester_dir, target_dir

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
        return "riscv32imac-unknown-none-elf"

def build_tester_firmware(config):
    base_dir, tester_dir, target_dir = _get_paths()
    rust_target = _get_rust_target()

    print(f"Building HCP2 Tester Lib (Rust) for {rust_target}...")
    try:
        cargo_cmd = ["cargo"]
        extra_args = []
        if rust_target.startswith("xtensa-"):
            cargo_cmd.append("+esp")
            extra_args = ["-Z", "build-std=core"]
        
        subprocess.run(
            cargo_cmd + ["build", "--release", "--target", rust_target] + extra_args, 
            cwd=tester_dir, check=True
        )
    except Exception as e:
        print(f"Error building Tester lib: {e}")
        raise RuntimeError(f"Rust Tester library build failed for {rust_target}")

    lib_path = os.path.join(target_dir, rust_target, "release", "libhcp2_tester_lib.a")
    
    # We copy it to a build-relative path so PIO finds it
    output_path = CORE.relative_build_path("tester-firmware/libhcp2_tester_lib.a")
    try:
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        shutil.copy(lib_path, output_path)
    except Exception as e:
        print(f"Error copying Rust artifacts: {e}")
        raise e
        
    return os.path.dirname(output_path)
