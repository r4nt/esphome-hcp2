#pragma once

#include "esphome/core/component.h"
#include "esphome/components/uart/uart.h"
#include "esphome/components/cover/cover.h"
#include "esphome/components/switch/switch.h"

namespace esphome {
namespace hcp_tester {

struct TesterState {
    float current_pos;
    float target_pos;
    bool light_on;
    bool vent_on;
    uint8_t last_action;
};

class HCPTester : public Component, public uart::UARTDevice {
 public:
  void setup() override;
  void loop() override;
  void dump_config() override;

  void set_flow_control_pin(GPIOPin *pin) { flow_control_pin_ = pin; }
  
  void set_target_position(float pos);
  void toggle_light();

  GPIOPin *get_flow_control_pin() { return flow_control_pin_; }

  TesterState state_;

 protected:
  GPIOPin *flow_control_pin_{nullptr};
};

class HCPTesterCover : public cover::Cover, public Component {
 public:
  void setup() override;
  void loop() override;
  void dump_config() override;
  void set_tester(HCPTester *tester) { tester_ = tester; }
  
 protected:
  HCPTester *tester_{nullptr};
  void control(const cover::CoverCall &call) override;
  cover::CoverTraits get_traits() override;
};

class HCPTesterSwitch : public switch_::Switch, public Component {
 public:
  void setup() override;
  void loop() override;
  void dump_config() override;
  void set_tester(HCPTester *tester) { tester_ = tester; }

 protected:
  HCPTester *tester_{nullptr};
  void write_state(bool state) override;
};

}  // namespace hcp_tester
}  // namespace esphome