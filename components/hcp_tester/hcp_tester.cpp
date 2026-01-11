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
    };

    void hcp_tester_init();
    void hcp_tester_poll(const TesterHalC *hal, TesterState *state);
    void hcp_tester_set_control(float target_pos, bool toggle_light);
}

static int32_t proxy_read_uart(void *ctx, uint8_t *buf, size_t len) {
    HCPTester *tester = static_cast<HCPTester *>(ctx);
    size_t i = 0;
    while (i < len && tester->available()) {
        if (!tester->read_byte(&buf[i]))
            break;
        i++;
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
    
    tester->write_array(buf, len);
    tester->flush(); // Calls uart_wait_tx_done on ESP32 (waits for Shift Register)

    return len;
}

static uint32_t proxy_now_ms() {
    return millis();
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
    // Poll state from tester component? 
    // Or relying on update_entities call from HCPTester loop would be better, 
    // but here we just poll the public state of the parent.
    if (tester_ == nullptr) return;
    
    // Map 0-200 to 0.0-1.0
    // Actually, HCP sends 0-200.
    float pos = tester_->state_.current_pos / 200.0f;
    if (this->position != pos) {
        this->position = pos;
        this->publish_state();
    }
    
    // State mapping
    // Simple for now: if moving, show moving.
    // Ideally we map DriveState enum.
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
    // Only support toggling really, but let's try setting
    if (state != tester_->state_.light_on) {
        tester_->toggle_light();
    }
}

}  // namespace hcp_tester
}  // namespace esphome
