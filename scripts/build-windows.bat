@echo off
setlocal enabledelayedexpansion

:: Rust MCP Server Build Script for Windows
:: https://github.com/yuunnn-w/Rust-MCP-Server

echo ========================================
echo   Rust MCP Server Build Script (Windows)  
echo ========================================
echo.

:: Get script directory and project root
set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."
cd /d "%PROJECT_ROOT%"

:: Check required tools
where rustc >nul 2>nul
if %errorlevel% neq 0 (
    echo Error: rustc not found, please install Rust
    echo Visit: https://rustup.rs/
    exit /b 1
)

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo Error: cargo not found, please install Rust
    exit /b 1
)

echo [OK] Build environment check passed
echo.

:: Check for download tool
set "DOWNLOAD_TOOL=none"
where curl >nul 2>nul
if %errorlevel% equ 0 (
    set "DOWNLOAD_TOOL=curl"
) else (
    where wget >nul 2>nul
    if !errorlevel! equ 0 (
        set "DOWNLOAD_TOOL=wget"
    )
)

:: Create static directory if it doesn't exist
if not exist "src\web\static" mkdir "src\web\static"

:: Download Chart.js if not exists
if not "%DOWNLOAD_TOOL%"=="none" (
    echo Downloading frontend dependencies...
    cd "src\web\static"
    
    if not exist "chart.min.js" (
        echo Downloading Chart.js...
        if "%DOWNLOAD_TOOL%"=="curl" (
            curl -# -L https://cdn.jsdelivr.net/npm/chart.js@4.5.1/dist/chart.umd.min.js -o chart.min.js
        ) else (
            wget -q --show-progress https://cdn.jsdelivr.net/npm/chart.js@4.5.1/dist/chart.umd.min.js -O chart.min.js
        )
        if !errorlevel! equ 0 (
            echo [OK] Chart.js downloaded successfully
        ) else (
            echo [Warning] Failed to download Chart.js, will try to use local copy
        )
    ) else (
        echo Chart.js already exists, skipping download
    )
    
    cd ..\..\..
)

echo.
echo Building Rust MCP Server...
echo.

:: Build the main project
set RUSTFLAGS=-C target-feature=+crt-static
cargo build --release

:: Check if build succeeded
if not exist "target\release\rust-mcp-server.exe" (
    echo [Error] Build failed: rust-mcp-server.exe not found
    exit /b 1
)

:: Copy main executable to project root
copy "target\release\rust-mcp-server.exe" ".\rust-mcp-server.exe" >nul
echo [OK] Main server executable copied: .\rust-mcp-server.exe

echo.
echo ========================================
echo      Build completed successfully!       
echo ========================================
echo.
echo Quick Start:
echo   rust-mcp-server.exe                    Start with default settings
echo   rust-mcp-server.exe --webui-port 8080  Start with custom WebUI port
echo.
echo Testing with llama.cpp:
echo   # Start this server first
echo   rust-mcp-server.exe --mcp-transport http --mcp-port 8080
echo.
echo   # Then start llama-server with MCP config:
echo   llama-server.exe -m your-model.gguf --mcp-config-url http://localhost:8080/config
echo.
echo Help:
echo   rust-mcp-server.exe --help             Show all available options
echo.
echo Documentation:
echo   README.md                              English documentation
echo   README-zh.md                           Chinese documentation
echo   docs\                                  Detailed documentation
echo.

pause
