import esphome.codegen as cg
import esphome.config_validation as cv
from esphome.components import cover
from esphome.const import CONF_ID
from .. import hcp_tester_ns, HCPTester

HCPTesterCover = hcp_tester_ns.class_("HCPTesterCover", cover.Cover, cg.Component)

CONF_HCP_TESTER_ID = "hcp_tester_id"

CONFIG_SCHEMA = cover.cover_schema(HCPTesterCover).extend({
    cv.Required(CONF_HCP_TESTER_ID): cv.use_id(HCPTester),
}).extend(cv.COMPONENT_SCHEMA)

async def to_code(config):
    var = cg.new_Pvariable(config[CONF_ID])
    await cover.register_cover(var, config)
    await cg.register_component(var, config)
    
    parent = await cg.get_variable(config[CONF_HCP_TESTER_ID])
    cg.add(var.set_tester(parent))
