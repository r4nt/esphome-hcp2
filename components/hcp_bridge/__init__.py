import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.components import uart
from esphome.const import CONF_ID, CONF_FLOW_CONTROL_PIN
from .build_hooks import is_lp_mode, build_lp_firmware, build_hp_firmware
from esphome.core import CORE

CONF_CORE = "core"

hcp_bridge_ns = cg.esphome_ns.namespace("hcp_bridge")
HCPBridge = hcp_bridge_ns.class_("HCPBridge", cg.Component, uart.UARTDevice)

CONFIG_SCHEMA = cv.Schema({
    cv.GenerateID(): cv.declare_id(HCPBridge),
    cv.Optional(CONF_CORE, default="lp"): cv.one_of("lp", "hp", lower=True),
    cv.Optional(CONF_FLOW_CONTROL_PIN, default=2): cv.int_,
}).extend(uart.UART_DEVICE_SCHEMA).extend(cv.COMPONENT_SCHEMA)

async def to_code(config):
    var = cg.new_Pvariable(config[CONF_ID])
    await cg.register_component(var, config)
    await uart.register_uart_device(var, config)
    
    cg.add(var.set_core_config(
        config[CONF_CORE] == "lp",
        config[CONF_FLOW_CONTROL_PIN]
    ))
    
    # Trigger the appropriate Rust build
    if is_lp_mode(config):
        build_lp_firmware(config)
        cg.add_build_flag("-DUSE_HCP_LP_MODE")
    else:
        build_hp_firmware(config)
        cg.add_build_flag("-L" + str(CORE.relative_build_path("hp-firmware")))
        cg.add_build_flag("-lhcp2_hp_lib")