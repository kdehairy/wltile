#!/usr/bin/env bash
# Collects combined unit + integration coverage for the wltile crate.
#
# Run inside the coverage container (see tests/Dockerfile.coverage). The
# integration harness spawns the instrumented `wltile` binary as a subprocess;
# because it inherits LLVM_PROFILE_FILE from cargo-llvm-cov, that subprocess
# coverage is captured and merged with the in-process (unit) coverage.
set -euo pipefail

# Only report on the crate's own sources — not the test harness or generated code.
IGNORE='tests/|target/'

cargo llvm-cov clean --workspace

# Unit tests (in-process).
cargo llvm-cov --no-report --bin wltile

# Integration tests: drives the real binary against a headless sway. Bounded so a
# stall can't hang the whole coverage run.
timeout --kill-after=30s 15m cargo llvm-cov --no-report --test integration

mkdir -p /output

echo "===================== COVERAGE SUMMARY ====================="
cargo llvm-cov report --ignore-filename-regex "$IGNORE" --summary-only | tee /output/summary.txt

cargo llvm-cov report --ignore-filename-regex "$IGNORE" --html --output-dir /output
cargo llvm-cov report --ignore-filename-regex "$IGNORE" --lcov --output-path /output/lcov.info

# The report is written as the container's uid (testuser); make it readable and
# traversable by all.
chmod -R a+rX /output/html
chmod a+r /output/summary.txt /output/lcov.info

echo "HTML report written to /output/html/index.html (target/coverage/ on the host)"
