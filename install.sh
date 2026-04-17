#!/bin/bash
set -e

cargo build --release
cp target/release/intern /usr/local/bin/intern
echo "intern installed to /usr/local/bin/intern"
