#include "hcp_bridge.h"
#include "esphome/core/log.h"
#include "esphome/core/hal.h"

#ifdef USE_HCP_LP_MODE
extern uint8_t lp_firmware_bin[];
extern size_t lp_firmware_bin_size;
#endif

#include <driver/gpio.h>
#include <soc/soc_caps.h>
#include <algorithm>

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
        void (*log)(void *ctx, const uint8_t *msg, size_t len);
    };

#ifndef USE_HCP_LP_MODE
    void hcp_hp_init();
    void hcp_hp_poll(const HcpHalC *hal, hcp2::SharedData *shared);
#endif
}

// Proxy implementations
#ifndef USE_HCP_LP_MODE
static void log_hex(const char *label, const uint8_t *buf, size_t len) {
#if ESPHOME_LOG_LEVEL >= ESPHOME_LOG_LEVEL_DEBUG
    if (len == 0) return;
    const size_t MAX_BYTES_PER_LINE = 64;
    char hex_buf[MAX_BYTES_PER_LINE * 3 + 1];
    
    size_t printed = 0;
    while (printed < len) {
        size_t chunk = std::min(len - printed, MAX_BYTES_PER_LINE);
        for (size_t i = 0; i < chunk; i++) {
            sprintf(hex_buf + i * 3, "%02X ", buf[printed + i]);
        }
        hex_buf[chunk * 3] = '\0';
        ESP_LOGD(TAG, "%s: %s", label, hex_buf);
        printed += chunk;
    }
#endif
}

static int32_t proxy_read_uart(void *ctx, uint8_t *buf, size_t len) {
    HCPBridge *bridge = static_cast<HCPBridge *>(ctx);
    size_t i = 0;
    while (i < len && bridge->available()) {
        if (!bridge->read_byte(&buf[i]))
            break;
        i++;
    }
    
    if (i > 0) {
        log_hex("RX", buf, i);
    }
    
    return i;
}

static int32_t proxy_write_uart(void *ctx, const uint8_t *buf, size_t len) {
    HCPBridge *bridge = static_cast<HCPBridge *>(ctx);
    log_hex("TX", buf, len);
    
    bridge->write_array(buf, len);
    return len;
}

static void proxy_set_tx_enable(void *ctx, bool enable) {
    HCPBridge *bridge = static_cast<HCPBridge *>(ctx);
    if (bridge->get_de_pin())
        bridge->get_de_pin()->digital_write(enable);
}

static uint32_t proxy_now_ms() {
    return millis();
}

static void proxy_sleep_ms(uint32_t ms) {
    delay(ms);
}

static void proxy_log(void *ctx, const uint8_t *msg, size_t len) {
    ESP_LOGD(TAG, "Rust: %.*s", len, (const char *)msg);
}
#endif

void HCPBridge::setup() {
  ESP_LOGCONFIG(TAG, "Setting up HCP Bridge...");

#ifdef USE_HCP_LP_MODE
  // Shared memory is at fixed address 0x50003000 in LP RAM
  shared_data_ = reinterpret_cast<hcp2::SharedData *>(0x50003000);
#else
  shared_data_ = new hcp2::SharedData();
#endif

  // Initialize shared memory
  if (try_lock()) {
    shared_data_->owner_flag = hcp2::OWNER_FREE;
    shared_data_->command_request = hcp2::CMD_NONE;
    shared_data_->last_update_ts = 0;
    unlock();
  }
  
#if defined(USE_HCP_LP_MODE)
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
#else
  // Initialize DE pin
  if (de_pin_) {
      de_pin_->setup();
      de_pin_->digital_write(false);
  }

  start_hp_task();
#endif
}

#ifndef USE_HCP_LP_MODE
void HCPBridge::start_hp_task() {
  ESP_LOGI(TAG, "Starting HP Core Task...");
  BaseType_t res;
  
  #if defined(SOC_CPU_CORES_NUM) && (SOC_CPU_CORES_NUM == 1)
    // Single core environment
    res = xTaskCreatePinnedToCore(hp_core_task, "hcp_hp_task", 4096, this, 5, &hp_task_handle_, 0);
  #else
    // Dual core environment - Pin to Core 1 (App Core)
    res = xTaskCreatePinnedToCore(hp_core_task, "hcp_hp_task", 4096, this, 5, &hp_task_handle_, 1);
  #endif

  if (res != pdPASS) {
      ESP_LOGE(TAG, "Failed to create HP Core Task! Error: %d", res);
  } else {
      ESP_LOGI(TAG, "HP Core Task launched successfully");
  }
}

void HCPBridge::hp_core_task(void *arg) {
  HCPBridge *self = static_cast<HCPBridge *>(arg);
  
  // Initialize Rust driver
  hcp_hp_init();
  
  // Prepare HAL struct
  HcpHalC hal_c = {
      .ctx = self,
      .read_uart = proxy_read_uart,
      .write_uart = proxy_write_uart,
      .set_tx_enable = proxy_set_tx_enable,
      .now_ms = proxy_now_ms,
      .sleep_ms = proxy_sleep_ms,
      .log = proxy_log,
  };

  ESP_LOGI(TAG, "Entering HP Core Loop...");
  while (true) {
      hcp_hp_poll(&hal_c, self->shared_data_);
      delay(1); // Yield/Sleep to prevent WDT
  }
  vTaskDelete(NULL);
}
#endif

void HCPBridge::loop() {
}

void HCPBridge::dump_config() {
  ESP_LOGCONFIG(TAG, "HCP Bridge:");
  ESP_LOGCONFIG(TAG, "  Shared Memory Address: %p", shared_data_);
#ifdef USE_HCP_LP_MODE
  ESP_LOGCONFIG(TAG, "  Flow Control Pin: %d", de_pin_);
#else
  LOG_PIN("  Flow Control Pin: ", de_pin_);
#endif
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
