.PHONY: all build release debug test check flash clean
all:
	$(MAKE) clean
	$(MAKE) flash
build: release

release:
	cargo build --locked --release --target thumbv7em-none-eabihf --features firmware
	arm-none-eabi-size target/thumbv7em-none-eabihf/release/gyro-breakout

debug:
	cargo build --locked --target thumbv7em-none-eabihf --features firmware
	arm-none-eabi-size target/thumbv7em-none-eabihf/debug/gyro-breakout

test:
	cargo test --locked

check: test build
	cargo fmt --check

flash: build
	openocd -f board/stm32f429discovery.cfg -c "program target/thumbv7em-none-eabihf/release/gyro-breakout verify reset exit"
	arm-none-eabi-size target/thumbv7em-none-eabihf/release/gyro-breakout

clean:
	cargo clean
