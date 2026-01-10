import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.components import uart
from esphome import pins
from esphome.const import CONF_ID, CONF_FLOW_CONTROL_PIN
from esphome.core import CORE

hcp_tester_ns = cg.esphome_ns.namespace("hcp_tester")
HCPTester = hcp_tester_ns.class_("HCPTester", cg.Component, uart.UARTDevice)

CONFIG_SCHEMA = cv.Schema({
    cv.GenerateID(): cv.declare_id(HCPTester),
    cv.Optional(CONF_FLOW_CONTROL_PIN): pins.gpio_output_pin_schema,
}).extend(uart.UART_DEVICE_SCHEMA).extend(cv.COMPONENT_SCHEMA)

async def to_code(config):
    var = cg.new_Pvariable(config[CONF_ID])
    await cg.register_component(var, config)
    await uart.register_uart_device(var, config)

    if CONF_FLOW_CONTROL_PIN in config:
        pin = await cg.gpio_pin_expression(config[CONF_FLOW_CONTROL_PIN])
        cg.add(var.set_flow_control_pin(pin))

    # Link Rust Library
    from .build_hooks import build_tester_firmware
    lib_path = build_tester_firmware(config)
    
    cg.add_build_flag("-L" + str(lib_path))
    cg.add_build_flag("-lhcp2_tester_lib")
