@echo off
REM fnva - 跨平台环境切换工具 Windows 启动脚本
REM 根据当前平台和架构选择对应的二进制文件并执行

setlocal enabledelayedexpansion

REM 获取脚本所在目录
set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..

REM 检测架构
set ARCH=%PROCESSOR_ARCHITECTURE%
if /i "%ARCH%"=="AMD64" (
    set CPU=x64
) else if /i "%ARCH%"=="ARM64" (
    set CPU=arm64
) else (
    echo Warning: Unrecognized architecture %ARCH%, falling back to x64 >&2
    set CPU=x64
)

REM 构建二进制文件路径
set PLATFORM_DIR=win32-%CPU%
set BINARY_PATH=%PROJECT_ROOT%platforms\%PLATFORM_DIR%\fnva.exe

REM 检查二进制文件是否存在
if not exist "%BINARY_PATH%" (
    echo Error: Binary file not found: %BINARY_PATH% >&2
    echo Please run 'npm run build' to build the binary >&2
    exit /b 1
)

REM 执行二进制文件，传递所有参数
"%BINARY_PATH%" %*