import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.const import CONF_ID
from .build_hooks import build_rust_firmware

hcp_bridge_ns = cg.esphome_ns.namespace("hcp_bridge")
HCPBridge = hcp_bridge_ns.class_("HCPBridge", cg.Component)

CONFIG_SCHEMA = cv.Schema({
    cv.GenerateID(): cv.declare_id(HCPBridge),
}).extend(cv.COMPONENT_SCHEMA)

async def to_code(config):
    # Trigger the automated build of the Rust firmware
    build_rust_firmware(config)

    var = cg.new_PVar(config[CONF_ID])
    await cg.register_component(var, config)
    
    # Add ESP-IDF dependencies for LP Core
    cg.add_library("ulp", None)
    cg.add_build_flag("-DUSE_ESP32_VARIANT_ESP32C6")
