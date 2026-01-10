import esphome.codegen as cg
import esphome.config_validation as cv
from esphome import pins
from esphome.components import uart
from esphome.const import CONF_ID, CONF_FLOW_CONTROL_PIN, CONF_TX_PIN, CONF_RX_PIN
from .build_hooks import is_lp_mode, build_lp_firmware, build_hp_firmware
from esphome.core import CORE

CONF_CORE = "core"

hcp_bridge_ns = cg.esphome_ns.namespace("hcp_bridge")
HCPBridge = hcp_bridge_ns.class_("HCPBridge", cg.Component, uart.UARTDevice)

# Base fields common to both
BASE_SCHEMA = cv.Schema({
    cv.GenerateID(): cv.declare_id(HCPBridge),
}).extend(cv.COMPONENT_SCHEMA)

# LP Mode Schema: Allows pins, no UART component required
LP_SCHEMA = BASE_SCHEMA.extend({
    cv.Optional(CONF_CORE, default="lp"): cv.one_of("lp", lower=True),
    cv.Optional(CONF_FLOW_CONTROL_PIN, default=2): cv.int_,
    cv.Optional(CONF_TX_PIN): cv.int_,
    cv.Optional(CONF_RX_PIN): cv.int_,
})

# HP Mode Schema: Requires UART component
HP_SCHEMA = BASE_SCHEMA.extend({
    cv.Required(CONF_CORE): cv.one_of("hp", lower=True),
    cv.Optional(CONF_FLOW_CONTROL_PIN): pins.gpio_output_pin_schema,
}).extend(uart.UART_DEVICE_SCHEMA)

CONFIG_SCHEMA = cv.Any(LP_SCHEMA, HP_SCHEMA)

async def to_code(config):
    var = cg.new_Pvariable(config[CONF_ID])
    await cg.register_component(var, config)
    
    # Trigger the appropriate Rust build
    if is_lp_mode(config):
        cg.add(var.set_flow_control_pin(
            config[CONF_FLOW_CONTROL_PIN]
        ))
        build_lp_firmware(config)
        cg.add_build_flag("-DUSE_HCP_LP_MODE")
    else:
        if CONF_FLOW_CONTROL_PIN in config:
            pin = await cg.gpio_pin_expression(config[CONF_FLOW_CONTROL_PIN])
            cg.add(var.set_flow_control_pin(pin))

        build_hp_firmware(config)
        await uart.register_uart_device(var, config)
        cg.add_build_flag("-L" + str(CORE.relative_build_path("hp-firmware")))
        cg.add_build_flag("-lhcp2_hp_lib")