@echo off
REM fnva - Windows CMD 启动脚本

setlocal

REM 检测平台和架构
set OS=win32
if "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
    set ARCH=x64
) else if "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
    set ARCH=arm64
) else (
    set ARCH=x64
)

REM 构建二进制文件路径
set PLATFORM_DIR=%OS%-%ARCH%
set BINARY_PATH=%~dp0..\platforms\%PLATFORM_DIR%\fnva.exe

REM 如果分层结构不存在，尝试扁平结构
if not exist "%BINARY_PATH%" (
    set BINARY_PATH=%~dp0..\platforms\fnva.exe
)

REM 检查二进制文件是否存在
if not exist "%BINARY_PATH%" (
    echo Error: Binary file not found
    echo Tried paths:
    echo   1. %~dp0..\platforms\%PLATFORM_DIR%\fnva.exe
    echo   2. %~dp0..\platforms\fnva.exe
    echo Please run 'npm run build' to build the binary
    exit /b 1
)

REM 执行二进制文件
"%BINARY_PATH%" %*

endlocal