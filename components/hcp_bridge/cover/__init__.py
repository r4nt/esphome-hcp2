import esphome.codegen as cg
import esphome.config_validation as cv
import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.components import cover
from esphome.const import CONF_ID
from .. import HCPBridge, hcp_bridge_ns

CONF_HCP_BRIDGE_ID = "hcp_bridge_id"

HCPCover = hcp_bridge_ns.class_("HCPCover", cover.Cover, cg.Component)

CONFIG_SCHEMA = cover.cover_schema(HCPCover).extend({
    cv.GenerateID(CONF_HCP_BRIDGE_ID): cv.use_id(HCPBridge),
}).extend(cv.COMPONENT_SCHEMA)

async def to_code(config):
    var = cg.new_Pvariable(config[CONF_ID])
    await cg.register_component(var, config)
    await cover.register_cover(var, config)
    
    bridge = await cg.get_variable(config[CONF_HCP_BRIDGE_ID])
    cg.add(var.set_bridge(bridge))
