#include "hcp_tester.h"
#include "esphome/core/log.h"
#include "esphome/core/hal.h"

namespace esphome {
namespace hcp_tester {

static const char *const TAG = "hcp_tester";

extern "C" {
    struct TesterHalC {
        void *ctx;
        int32_t (*read_uart)(void *ctx, uint8_t *buf, size_t len);
        int32_t (*write_uart)(void *ctx, const uint8_t *buf, size_t len);
        void (*set_tx_enable)(void *ctx, bool enable);
        uint32_t (*now_ms)();
        void (*log)(void *ctx, const uint8_t *msg, size_t len);
    };

    void hcp_tester_init();
    void hcp_tester_poll(const TesterHalC *hal, TesterState *state);
    void hcp_tester_set_control(float target_pos, bool toggle_light);
}

// Helper to log hex buffers using ESPHome's logger
// This ensures it respects ESPHOME_LOG_LEVEL and prints correctly to the configured sink.
static void log_hex(const char *label, const uint8_t *buf, size_t len) {
#if ESPHOME_LOG_LEVEL >= ESPHOME_LOG_LEVEL_DEBUG
    if (len == 0) return;
    // Buffer for "XX XX XX ..." (3 chars per byte + null terminator)
    // 64 bytes max per line to avoid massive stack allocation
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
    HCPTester *tester = static_cast<HCPTester *>(ctx);
    size_t i = 0;
    while (i < len && tester->available()) {
        if (!tester->read_byte(&buf[i]))
            break;
        i++;
    }
    if (i > 0) {
        log_hex("RX", buf, i);
    }
    return i;
}

static void proxy_set_tx_enable(void *ctx, bool enable) {
    HCPTester *tester = static_cast<HCPTester *>(ctx);
    if (tester->get_flow_control_pin()) {
        tester->get_flow_control_pin()->digital_write(enable);
    }
}

static int32_t proxy_write_uart(void *ctx, const uint8_t *buf, size_t len) {
    HCPTester *tester = static_cast<HCPTester *>(ctx);
    
    log_hex("TX", buf, len);
    
    tester->write_array(buf, len);
    tester->flush(); 

    return len;
}

static uint32_t proxy_now_ms() {
    return millis();
}

static void proxy_log(void *ctx, const uint8_t *msg, size_t len) {
    ESP_LOGD(TAG, "Rust: %.*s", len, (const char *)msg);
}

void HCPTester::setup() {
    ESP_LOGI(TAG, "Initializing HCP Tester...");
    if (flow_control_pin_) {
        flow_control_pin_->setup();
        flow_control_pin_->digital_write(false);
    }
    hcp_tester_init();
}

void HCPTester::loop() {
    TesterHalC hal = {
        .ctx = this,
        .read_uart = proxy_read_uart,
        .write_uart = proxy_write_uart,
        .set_tx_enable = proxy_set_tx_enable,
        .now_ms = proxy_now_ms,
        .log = proxy_log,
    };

    hcp_tester_poll(&hal, &state_);
}

void HCPTester::dump_config() {
    ESP_LOGCONFIG(TAG, "HCP Tester");
    LOG_PIN("  Flow Control Pin: ", flow_control_pin_);
}

void HCPTester::set_target_position(float pos) {
    hcp_tester_set_control(pos, false);
}

void HCPTester::toggle_light() {
    hcp_tester_set_control(state_.target_pos, true);
}

// Cover Implementation
void HCPTesterCover::setup() { }
void HCPTesterCover::loop() {
    if (tester_ == nullptr) return;
    
    float pos = tester_->state_.current_pos / 200.0f;
    if (this->position != pos) {
        this->position = pos;
        this->publish_state();
    }
}

void HCPTesterCover::dump_config() { LOG_COVER("", "HCP Tester Cover", this); }

void HCPTesterCover::control(const cover::CoverCall &call) {
    if (call.get_position().has_value()) {
        float pos = *call.get_position();
        tester_->set_target_position(pos * 200.0f);
    }
}

cover::CoverTraits HCPTesterCover::get_traits() {
    auto traits = cover::CoverTraits();
    traits.set_supports_toggle(true);
    return traits;
}

// Switch Implementation
void HCPTesterSwitch::setup() {}
void HCPTesterSwitch::loop() {
    if (tester_ == nullptr) return;
    if (this->state != tester_->state_.light_on) {
        this->publish_state(tester_->state_.light_on);
    }
}
void HCPTesterSwitch::dump_config() { LOG_SWITCH("", "HCP Tester Light", this); }

void HCPTesterSwitch::write_state(bool state) {
    if (state != tester_->state_.light_on) {
        tester_->toggle_light();
    }
}

}  // namespace hcp_tester
}  // namespace esphome
