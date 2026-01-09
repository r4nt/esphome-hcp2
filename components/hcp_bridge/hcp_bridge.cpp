#include "hcp_bridge.h"
#include "esphome/core/log.h"
#include "esphome/core/hal.h"
#include "hcp2_lp_bin.h"
#include <driver/uart.h>
#include <driver/gpio.h>

namespace esphome {
namespace hcp_bridge {

static const char *const TAG = "hcp_bridge";

void HCPBridge::setup() {
  ESP_LOGCONFIG(TAG, "Setting up HCP Bridge...");

  if (use_lp_core_) {
    // Shared memory is at fixed address 0x50002000 in LP RAM
    shared_data_ = reinterpret_cast<hcp2::SharedData *>(0x50002000);
  } else {
    // For HP mode, allocate shared memory on the heap
    shared_data_ = new hcp2::SharedData();
  }

  // Initialize shared memory
  if (try_lock()) {
    shared_data_->owner_flag = hcp2::OWNER_FREE;
    shared_data_->command_request = hcp2::CMD_NONE;
    shared_data_->last_update_ts = 0;
    unlock();
  }

#ifdef USE_ESP32_VARIANT_ESP32C6
  if (use_lp_core_) {
    ESP_LOGI(TAG, "Starting LP Core...");
    esp_err_t err = ulp_lp_core_load_binary(lp_firmware_bin, lp_firmware_bin_size);
    if (err != ESP_OK) {
      ESP_LOGE(TAG, "Failed to load LP firmware: %d", err);
      return;
    }

    ulp_lp_core_cfg_t cfg = {
      .wakeup_source = ULP_LP_CORE_WAKEUP_SOURCE_HP_CPU,
    };
    err = ulp_lp_core_run(&cfg);
    if (err != ESP_OK) {
      ESP_LOGE(TAG, "Failed to run LP core: %d", err);
    }
  } else {
    ESP_LOGI(TAG, "Starting HP Core Task...");
    xTaskCreate(hp_core_task, "hcp_hp_task", 4096, this, 5, &hp_task_handle_);
  }
#else
  ESP_LOGW(TAG, "LP Core only supported on ESP32-C6. Running in stub mode.");
#endif
}

void HCPBridge::hp_core_task(void *arg) {
  HCPBridge *self = static_cast<HCPBridge *>(arg);
  
  uart_config_t uart_config = {
      .baud_rate = 57600,
      .data_bits = UART_DATA_8_BITS,
      .parity = UART_PARITY_EVEN,
      .stop_bits = UART_STOP_BITS_1,
      .flow_ctrl = UART_HW_FLOWCTRL_DISABLE,
      .source_clk = UART_SCLK_DEFAULT,
  };
  uart_driver_install(UART_NUM_1, 256, 0, 0, NULL, 0);
  uart_param_config(UART_NUM_1, &uart_config);
  uart_set_pin(UART_NUM_1, self->tx_pin_, self->rx_pin_, UART_PIN_NO_CHANGE, UART_PIN_NO_CHANGE);
  
  gpio_reset_pin((gpio_num_t)self->de_pin_);
  gpio_set_direction((gpio_num_t)self->de_pin_, GPIO_MODE_OUTPUT);
  gpio_set_level((gpio_num_t)self->de_pin_, 0);

  // Initialize Protocol (Rust)
  uint8_t proto_mem[128]; // Enough for Hcp2Protocol
  hcp2::hcp2_protocol_init(proto_mem);

  uint8_t rx_buf[128];
  uint8_t tx_buf[128];
  int rx_len = 0;
  
  while (true) {
    // Read UART with short timeout
    int len = uart_read_bytes(UART_NUM_1, rx_buf + rx_len, 1, 10 / portTICK_PERIOD_MS);
    if (len > 0) {
      rx_len += len;
    } else {
      // Timeout - dispatch frame if we have data
      if (rx_len > 0) {
        if (self->try_lock()) {
          self->shared_data_->owner_flag = hcp2::OWNER_LP; // HP acting as LP logic
          
          uintptr_t tx_len = hcp2::hcp2_protocol_dispatch(proto_mem, rx_buf, rx_len, tx_buf, sizeof(tx_buf), self->shared_data_, millis());
          
          if (tx_len > 0) {
            gpio_set_level((gpio_num_t)self->de_pin_, 1);
            uart_write_bytes(UART_NUM_1, (const char*)tx_buf, tx_len);
            uart_wait_tx_done(UART_NUM_1, 100); // Wait for flush
            esp_rom_delay_us(500); // Extra safety
            gpio_set_level((gpio_num_t)self->de_pin_, 0);
          }
          
          self->shared_data_->owner_flag = hcp2::OWNER_FREE;
          self->unlock();
        }
        rx_len = 0;
      }
    }
    
    // Prevent buffer overflow
    if (rx_len >= sizeof(rx_buf)) rx_len = 0;
  }
}

void HCPBridge::loop() {
  // We don't necessarily need to lock for reading single bytes, 
  // but for consistent multi-byte reads (ts) it's safer.
  // However, LP core updates frequently, so we don't want to block HP loop.
  // We'll just read directly since it's most efficient and mostly safe for single fields.
}

void HCPBridge::dump_config() {
  ESP_LOGCONFIG(TAG, "HCP Bridge:");
  ESP_LOGCONFIG(TAG, "  Shared Memory Address: %p", shared_data_);
}

bool HCPBridge::try_lock() {
  if (shared_data_->owner_flag == hcp2::OWNER_FREE) {
    shared_data_->owner_flag = hcp2::OWNER_HP;
    return true;
  }
  return false;
}

void HCPBridge::unlock() {
  shared_data_->owner_flag = hcp2::OWNER_FREE;
}

void HCPBridge::set_command(uint8_t command) {
  // Busy wait briefly for lock
  for (int i = 0; i < 100; i++) {
    if (try_lock()) {
      shared_data_->command_request = command;
      unlock();
      return;
    }
    esp_rom_delay_us(10);
  }
  ESP_LOGW(TAG, "Failed to acquire lock for command %d", command);
}

void HCPBridge::set_target_position(uint8_t position) {
  for (int i = 0; i < 100; i++) {
    if (try_lock()) {
      shared_data_->target_position = position;
      unlock();
      return;
    }
    esp_rom_delay_us(10);
  }
}

}  // namespace hcp_bridge
}  // namespace esphome