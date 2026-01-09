import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.const import CONF_ID, CONF_TX_PIN, CONF_RX_PIN
from .build_hooks import build_rust_firmware

CONF_CORE = "core"
CONF_FLOW_CONTROL_PIN = "flow_control_pin"

hcp_bridge_ns = cg.esphome_ns.namespace("hcp_bridge")
HCPBridge = hcp_bridge_ns.class_("HCPBridge", cg.Component)

CONFIG_SCHEMA = cv.Schema({
    cv.GenerateID(): cv.declare_id(HCPBridge),
    cv.Optional(CONF_CORE, default="lp"): cv.one_of("lp", "hp", lower=True),
    cv.Optional(CONF_TX_PIN, default=5): cv.int_,
    cv.Optional(CONF_RX_PIN, default=4): cv.int_,
    cv.Optional(CONF_FLOW_CONTROL_PIN, default=2): cv.int_,
}).extend(cv.COMPONENT_SCHEMA)

async def to_code(config):
    # Trigger the automated build of the Rust firmware (only needed for LP mode, but good to ensure logic is valid)
    build_rust_firmware(config)

    var = cg.new_Pvariable(config[CONF_ID])
    await cg.register_component(var, config)
    
    cg.add(var.set_core_config(
        config[CONF_CORE] == "lp",
        config[CONF_TX_PIN],
        config[CONF_RX_PIN],
        config[CONF_FLOW_CONTROL_PIN]
    ))
    
    # Add ESP-IDF dependencies
    cg.add_build_flag("-DUSE_ESP32_VARIANT_ESP32C6")
    
    # Link the HP static library
    cg.add_build_flag("-Lsrc/esphome/components/hcp_bridge")
    cg.add_build_flag("-lhcp2_hp_lib")
