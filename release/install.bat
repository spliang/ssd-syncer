@echo off
setlocal enabledelayedexpansion

REM ============================================================
REM SSD-Syncer Windows Installer
REM Place this script alongside ssd-syncer-windows.exe and run it.
REM After installation, you can use "sync <command>" globally.
REM ============================================================

echo.
echo  === SSD-Syncer Windows Installer ===
echo.

REM -- Determine the directory where this script resides --
set "SCRIPT_DIR=%~dp0"

REM -- Check that the binary exists in the same directory --
if not exist "%SCRIPT_DIR%ssd-syncer-windows.exe" (
    echo [ERROR] ssd-syncer-windows.exe not found in: %SCRIPT_DIR%
    echo Please place this script in the same directory as ssd-syncer-windows.exe
    pause
    exit /b 1
)

REM -- Target installation directory --
set "INSTALL_DIR=%USERPROFILE%\.ssd-syncer\bin"

echo Install directory: %INSTALL_DIR%
echo.

REM -- Create install directory --
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
    if errorlevel 1 (
        echo [ERROR] Failed to create directory: %INSTALL_DIR%
        pause
        exit /b 1
    )
)

REM -- Copy binary --
echo Copying ssd-syncer-windows.exe ...
copy /Y "%SCRIPT_DIR%ssd-syncer-windows.exe" "%INSTALL_DIR%\ssd-syncer.exe" >nul
if errorlevel 1 (
    echo [ERROR] Failed to copy binary.
    pause
    exit /b 1
)
echo   Done.

REM -- Create sync.bat wrapper --
echo Creating sync.bat wrapper ...
(
    echo @echo off
    echo "%%~dp0ssd-syncer.exe" %%*
) > "%INSTALL_DIR%\sync.bat"
echo   Done.

REM -- Check if INSTALL_DIR is already in PATH --
echo.
echo Checking PATH ...

REM Read current user PATH from registry
for /f "tokens=2,*" %%A in ('reg query "HKCU\Environment" /v Path 2^>nul') do set "USER_PATH=%%B"

if not defined USER_PATH set "USER_PATH="

echo !USER_PATH! | findstr /i /c:"%INSTALL_DIR%" >nul 2>nul
if %errorlevel%==0 (
    echo   PATH already contains %INSTALL_DIR%
) else (
    echo   Adding %INSTALL_DIR% to user PATH ...
    if "!USER_PATH!"=="" (
        setx PATH "%INSTALL_DIR%"
    ) else (
        setx PATH "!USER_PATH!;%INSTALL_DIR%"
    )
    if errorlevel 1 (
        echo   [WARNING] Failed to update PATH automatically.
        echo   Please manually add the following to your PATH:
        echo     %INSTALL_DIR%
    ) else (
        echo   Done.
    )
)

echo.
echo  === Installation Complete ===
echo.
echo  Binary:  %INSTALL_DIR%\ssd-syncer.exe
echo  Wrapper: %INSTALL_DIR%\sync.bat
echo.
echo  IMPORTANT: Please open a NEW terminal window for PATH changes to take effect.
echo.
echo  Usage:
echo    sync list
echo    sync init --name "my-windows"
echo    sync add --local "C:\Users\You\Documents\work" --ssd "WORK_SYNC"
echo    sync sync E:\
echo.

pause
