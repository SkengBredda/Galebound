#!/bin/sh
# Build and start the Galebound server. Extra options are passed through, e.g. --port 7778
cd "$(dirname "$0")" && exec cargo run --release -p galebound-server -- "$@"
