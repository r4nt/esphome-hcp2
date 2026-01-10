#pragma once

#include "esphome/core/component.h"
#include "esphome/core/helpers.h"
#ifndef USE_HCP_LP_MODE
#include "esphome/components/uart/uart.h"
#endif
#include "shared_data.h"

#if defined(USE_ESP32_VARIANT_ESP32C6) && defined(USE_HCP_LP_MODE)
#include "ulp_lp_core.h"
#endif

namespace esphome {
namespace hcp_bridge {

class HCPBridge : public Component
#ifndef USE_HCP_LP_MODE
    , public uart::UARTDevice
#endif
{
 public:
  void setup() override;
  void loop() override;
  void dump_config() override;

  void set_command(uint8_t command);
  void set_target_position(uint8_t position);
  
#ifdef USE_HCP_LP_MODE
  void set_flow_control_pin(int de) {
    de_pin_ = de;
  }

  int get_de_pin() const { return de_pin_; }
#else
  void set_flow_control_pin(GPIOPin *de) {
    de_pin_ = de;
  }

  GPIOPin *get_de_pin() const { return de_pin_; }
#endif

  const hcp2::SharedData *get_data() const { return shared_data_; }

 protected:
  hcp2::SharedData *shared_data_{nullptr};
  uint32_t last_sync_ms_{0};
#ifdef USE_HCP_LP_MODE
  int de_pin_{2};
#else
  GPIOPin *de_pin_{nullptr};
#endif
  
  TaskHandle_t hp_task_handle_{nullptr};
  
  bool try_lock();
  void unlock();
 
#ifndef USE_HCP_LP_MODE
  void start_hp_task();
  static void hp_core_task(void *arg);
#endif
};

}  // namespace hcp_bridge
}  // namespace esphome
