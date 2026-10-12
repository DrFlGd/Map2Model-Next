@echo off
setlocal EnableExtensions
echo.
echo Map2Model Next - PRIVATE Interlaken z17 FORMAT inspector
echo ---------------------------------------------------------
echo This does NOT generate terrain models or certify data licensing/datum.
echo.
if "%~1"=="" (
  echo Drag your interlaken-z17.pmtiles file onto this .cmd,
  echo or launch: Inspect-Interlaken.cmd "C:\path\interlaken-z17.pmtiles"
  echo.
  pause
  exit /b 2
)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Inspect-Interlaken.ps1" -ArchivePath "%~f1"
if errorlevel 1 (
  echo.
  echo TEST FAILED. Copy the error above when reporting to Agent 2.
  pause
  exit /b 1
)
echo.
echo Test completed. Copy the PASS lines above into the GitHub PR or chat.
pause
exit /b 0
