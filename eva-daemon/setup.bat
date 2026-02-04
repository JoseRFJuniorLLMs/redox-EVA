@echo off
REM Setup script for EVA Daemon development (Windows)

echo 🧠 EVA Daemon Setup Script
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

REM Check if we're in the right directory
if not exist "Cargo.toml" (
    echo ❌ Error: Run this script from the eva-daemon directory
    exit /b 1
)

echo.
echo Select phase to setup:
echo   1) Phase 1 - Network Testing
echo   2) Phase 2 - TLS/SSL (default)
echo.
set /p choice="Enter choice [1-2]: "

if "%choice%"=="1" goto phase1
if "%choice%"=="2" goto phase2
goto phase2

:phase1
echo.
echo 📦 Setting up Phase 1 (Network Testing)...
copy /Y Cargo_phase1.toml Cargo.toml >nul
copy /Y src\main_phase1.rs src\main.rs >nul
echo ✅ Phase 1 configured
echo.
echo To test Phase 1:
echo   cargo build --release
echo   cargo run
goto end

:phase2
echo.
echo 🔐 Setting up Phase 2 (TLS/SSL)...
echo ✅ Phase 2 configured (default)
echo.
echo To test Phase 2:
echo   cargo build --release
echo   cargo test
echo   cargo run
goto end

:end
echo.
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo ✅ Setup complete!
