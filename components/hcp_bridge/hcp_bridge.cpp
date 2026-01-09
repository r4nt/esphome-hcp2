#include "hcp_bridge.h"
#include "esphome/core/log.h"
#include "esphome/core/hal.h"
#include "hcp2_lp_bin.h"
#include <driver/uart.h>
#include <driver/gpio.h>

namespace esphome {
namespace hcp_bridge {

static const char *const TAG = "hcp_bridge";

// Rust FFI definitions
extern "C" {
    struct HcpHalC {
        void *ctx;
        int32_t (*read_uart)(void *ctx, uint8_t *buf, size_t len);
        int32_t (*write_uart)(void *ctx, const uint8_t *buf, size_t len);
        void (*set_tx_enable)(void *ctx, bool enable);
        uint32_t (*now_ms)();
        void (*sleep_ms)(uint32_t ms);
    };

    void hcp_run_hp_loop(const HcpHalC *hal, hcp2::SharedData *shared);
}

// Proxy implementations
static int32_t proxy_read_uart(void *ctx, uint8_t *buf, size_t len) {
    HCPBridge *bridge = static_cast<HCPBridge *>(ctx);
    // Use timeout=0 for non-blocking read
    int len_read = uart_read_bytes(UART_NUM_1, buf, len, 0);
    return len_read;
}

static int32_t proxy_write_uart(void *ctx, const uint8_t *buf, size_t len) {
    HCPBridge *bridge = static_cast<HCPBridge *>(ctx);
    return uart_write_bytes(UART_NUM_1, buf, len);
}

static void proxy_set_tx_enable(void *ctx, bool enable) {
    HCPBridge *bridge = static_cast<HCPBridge *>(ctx);
    gpio_set_level((gpio_num_t)bridge->get_de_pin(), enable ? 1 : 0);
}

static uint32_t proxy_now_ms() {
    return millis();
}

static void proxy_sleep_ms(uint32_t ms) {
    delay(ms);
}


void HCPBridge::setup() {
  ESP_LOGCONFIG(TAG, "Setting up HCP Bridge...");

  if (use_lp_core_) {
    // Shared memory is at fixed address 0x50003000 in LP RAM
    shared_data_ = reinterpret_cast<hcp2::SharedData *>(0x50003000);
  } else {
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
#if defined(USE_HCP_LP_MODE)
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
     // Fallback if configured for LP but flag is false
  }
#else
  // HP Mode
  ESP_LOGI(TAG, "Starting HP Core Task...");
  #if CONFIG_FREERTOS_UNICORE
  xTaskCreate(hp_core_task, "hcp_hp_task", 4096, this, 5, &hp_task_handle_);
  #else
    xTaskCreatePinnedToCore(hp_core_task, "hcp_hp_task", 4096, this, 5, &hp_task_handle_, 1);
  #endif
  
#endif
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

  // Prepare HAL struct
  HcpHalC hal_c = {
      .ctx = self,
      .read_uart = proxy_read_uart,
      .write_uart = proxy_write_uart,
      .set_tx_enable = proxy_set_tx_enable,
      .now_ms = proxy_now_ms,
      .sleep_ms = proxy_sleep_ms,
  };

  // Transfer control to Rust
  hcp_run_hp_loop(&hal_c, self->shared_data_);
  
  // Should never return
  vTaskDelete(NULL);
}

void HCPBridge::loop() {
    // ... same as before
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