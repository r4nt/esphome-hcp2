.PHONY: all esp32c6-lp esp32c6-hp esp32s3-hp tester-s3 tester-c6 test rust-test clean

export PATH := $(HOME)/.cargo/bin:/home/klimek/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$(PATH)

all: esp32c6-lp esp32c6-hp esp32s3-hp tester-s3 tester-c6

esp32c6-lp:
	esphome compile example_c6_lp.yaml

esp32c6-hp:
	esphome compile example_c6_hp.yaml

esp32s3-hp:
	esphome compile example_s3_hp.yaml

tester-s3:
	esphome compile example_tester_s3.yaml

tester-c6:
	esphome compile example_tester_c6.yaml

clean:
	rm -rf .esphome

rust-test:
	cd common && cargo test
	cd tester-firmware && cargo test --features std

test: rust-test
	rm -rf .esphome
	$(MAKE) all