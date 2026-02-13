@echo off
REM sync wrapper - uses cargo run to bypass Device Guard/WDAC policy
REM Source: https://github.com/spliang/ssd-syncer

set "CARGO_MANIFEST=%~dp0..\cli\Cargo.toml"
if not exist "%CARGO_MANIFEST%" (
    echo [Error] Cargo.toml not found at: %CARGO_MANIFEST%
    echo Please ensure the ssd-syncer source code is available.
    exit /b 1
)
cargo run --quiet --manifest-path "%CARGO_MANIFEST%" -- %*