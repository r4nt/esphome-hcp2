#pragma once

#include "esphome/components/cover/cover.h"
#include "../hcp_bridge.h"

namespace esphome {
namespace hcp_bridge {

class HCPCover : public cover::Cover, public Component {
 public:
  void set_bridge(HCPBridge *bridge) { bridge_ = bridge; }

  cover::CoverTraits get_traits() override {
    cover::CoverTraits traits;
    traits.set_supports_position(true);
    traits.set_supports_stop(true);
    return traits;
  }

  void setup() override {}
  
  void loop() override {
    const auto *data = bridge_->get_data();
    if (data == nullptr) return;

    float pos = static_cast<float>(data->current_position) / 200.0f;
    if (this->position != pos) {
      this->position = pos;
      this->publish_state();
    }

    // Map drive state to cover operation
    cover::CoverOperation op = cover::COVER_OPERATION_IDLE;
    switch (data->current_state) {
      case 0x01: // Opening
      case 0x05: // MoveHalf
      case 0x09: // MoveVenting
        op = cover::COVER_OPERATION_OPENING;
        break;
      case 0x02: // Closing
        op = cover::COVER_OPERATION_CLOSING;
        break;
    }
    
    if (this->current_operation != op) {
      this->current_operation = op;
      this->publish_state();
    }
  }

  void control(const cover::CoverCall &call) override {
    if (call.get_stop()) {
      bridge_->set_command(hcp2::CMD_STOP);
    } else if (call.get_position()) {
      float pos = *call.get_position();
      if (pos == 0.0f) {
        bridge_->set_command(hcp2::CMD_CLOSE);
      } else if (pos == 1.0f) {
        bridge_->set_command(hcp2::CMD_OPEN);
      } else {
        // HCP2 usually handles discrete buttons, but we can set target position if supported
        // For now, we'll just open/close based on direction
        if (pos > this->position) bridge_->set_command(hcp2::CMD_OPEN);
        else bridge_->set_command(hcp2::CMD_CLOSE);
      }
    }
  }

 protected:
  HCPBridge *bridge_;
};

}  // namespace hcp_bridge
}  // namespace esphome
