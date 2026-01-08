#pragma once

#include "esphome/components/switch/switch.h"
#include "../hcp_bridge.h"

namespace esphome {
namespace hcp_bridge {

class HCPVentSwitch : public switch_::Switch, public Component {
 public:
  void set_bridge(HCPBridge *bridge) { bridge_ = bridge; }

  void loop() override {
    const auto *data = bridge_->get_data();
    if (data == nullptr) return;

    bool venting = (data->current_state == 0x0A); // VentReached
    if (this->state != venting) {
      this->publish_state(venting);
    }
  }

  void write_state(bool state) override {
    if (state) {
      bridge_->set_command(hcp2::CMD_VENT);
    } else {
      bridge_->set_command(hcp2::CMD_CLOSE);
    }
  }

 protected:
  HCPBridge *bridge_;
};

}  // namespace hcp_bridge
}  // namespace esphome
