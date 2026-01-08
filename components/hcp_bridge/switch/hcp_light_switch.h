#pragma once

#include "esphome/components/switch/switch.h"
#include "../hcp_bridge.h"

namespace esphome {
namespace hcp_bridge {

class HCPLightSwitch : public switch_::Switch, public Component {
 public:
  void set_bridge(HCPBridge *bridge) { bridge_ = bridge; }

  void loop() override {
    const auto *data = bridge_->get_data();
    if (data == nullptr) return;

    if (this->state != data->light_on) {
      this->publish_state(data->light_on);
    }
  }

  void write_state(bool state) override {
    // HCP2 uses a toggle command for light
    if (state != this->state) {
      bridge_->set_command(hcp2::CMD_TOGGLE_LIGHT);
    }
  }

 protected:
  HCPBridge *bridge_;
};

}  // namespace hcp_bridge
}  // namespace esphome
