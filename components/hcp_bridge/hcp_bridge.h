#pragma once

#include "esphome/core/component.h"
#include "esphome/core/helpers.h"
#include "shared_data.h"

#ifdef USE_ESP32_VARIANT_ESP32C6
#include "ulp_lp_core.h"
#endif

#include <freertos/FreeRTOS.h>
#include <freertos/task.h>

namespace esphome {
namespace hcp_bridge {

class HCPBridge : public Component {
 public:
  void setup() override;
  void loop() override;
  void dump_config() override;

  void set_command(uint8_t command);
  void set_target_position(uint8_t position);
  
  void set_core_config(bool use_lp, int tx, int rx, int de) {
    use_lp_core_ = use_lp;
    tx_pin_ = tx;
    rx_pin_ = rx;
    de_pin_ = de;
  }

  const hcp2::SharedData *get_data() const { return shared_data_; }

 protected:
  hcp2::SharedData *shared_data_{nullptr};
  uint32_t last_sync_ms_{0};
  
  bool use_lp_core_{true};
  int tx_pin_{5};
  int rx_pin_{4};
  int de_pin_{2};
  
  TaskHandle_t hp_task_handle_{nullptr};
  
  bool try_lock();
  void unlock();
  
  static void hp_core_task(void *arg);
};

}  // namespace hcp_bridge
}  // namespace esphome
