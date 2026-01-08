import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.components import switch
from esphome.const import CONF_ID, CONF_TYPE
from .. import HCPBridge, hcp_bridge_ns

CONF_HCP_BRIDGE_ID = "hcp_bridge_id"

HCPLightSwitch = hcp_bridge_ns.class_("HCPLightSwitch", switch.Switch, cg.Component)
HCPVentSwitch = hcp_bridge_ns.class_("HCPVentSwitch", switch.Switch, cg.Component)

TYPES = {
    "light": HCPLightSwitch,
    "vent": HCPVentSwitch,
}

CONFIG_SCHEMA = switch.SWITCH_SCHEMA.extend({
    cv.GenerateID(): cv.declare_id(),
    cv.GenerateID(CONF_HCP_BRIDGE_ID): cv.use_id(HCPBridge),
    cv.Required(CONF_TYPE): cv.one_of(*TYPES, lower=True),
}).extend(cv.COMPONENT_SCHEMA)

async def to_code(config):
    var = cg.new_PVar(config[CONF_ID], TYPES[config[CONF_TYPE]]())
    await cg.register_component(var, config)
    await switch.register_switch(var, config)
    
    bridge = await cg.get_variable(config[CONF_HCP_BRIDGE_ID])
    cg.add(var.set_bridge(bridge))
