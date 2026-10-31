@echo off
title Galebound Server
echo Building and starting the Galebound server (the first build takes a few minutes)...
cargo run --release -p galebound-server -- %*
pause
