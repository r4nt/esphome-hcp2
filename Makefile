.PHONY: all esp32c6-lp esp32c6-hp esp32s3-hp test clean

export PATH := $(HOME)/.cargo/bin:/home/klimek/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$(PATH)

all: esp32c6-lp esp32c6-hp esp32s3-hp

esp32c6-lp:
	esphome compile example_c6_lp.yaml

esp32c6-hp:
	esphome compile example_c6_hp.yaml

esp32s3-hp:
	esphome compile example_s3_hp.yaml

clean:
	rm -rf .esphome

test:
	rm -rf .esphome
	$(MAKE) all
