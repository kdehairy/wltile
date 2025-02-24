.PHONY: build release run clean

build:
	@cargo build

release:
	@cargo build --release

run:
	@if [ -f ./target/logs.out ]; then rm ./target/logs.out; fi
	@cargo run

check:
	@cargo clippy

clean:
	@cargo clean


