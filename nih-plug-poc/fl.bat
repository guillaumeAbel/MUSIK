@echo off
echo [BAT] Script lance
set "NIH_LOG=%USERPROFILE%\MUSIK\nih-plug-poc/debug.txt"
echo [BAT] NIH_LOG = %NIH_LOG%
echo [BAT] Lancement de FL Studio...
pause
start "" "C:\Program Files\Image-Line\FL Studio 2025\FL64.exe"
