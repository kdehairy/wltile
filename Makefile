.PHONY: build release run clean integration-test

build:
	@cargo build

release:
	@cargo build --release

run:
	@if [ -f ./target/logs.out ]; then rm ./target/logs.out; fi
	@cargo run

check:
	@cargo clippy

integration-test:
	@docker build -t wltile-integration -f tests/Dockerfile .
	# --shm-size=1g is required: the suite runs ~11 sway compositors in
	# parallel, each allocating headless output framebuffers in /dev/shm.
	# Docker's default 64M shm fills up under this concurrency, and wlroots
	# then crashes with SIGBUS when it writes to an unbacked mmap'd buffer,
	# making swaymsg's create_output fail with "Unable to receive IPC response".
	#
	# --cap-add=SYS_NICE is required: the sway binary carries a
	# cap_sys_nice=ep file capability, which is only granted if SYS_NICE is
	# in the container's bounding set (it isn't, by Docker's default).
	@docker run --rm --shm-size=1g --cap-add=SYS_NICE wltile-integration

clean:
	@cargo clean


