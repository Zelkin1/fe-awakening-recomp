@echo off
rem Fire Emblem: Awakening, recompiled ahead of time by 3dsrecomp and run by Zakuro.
rem Saves, mods and save states live in userdata\ next to this file.
rem Zakuro keeps its window/controls settings in %APPDATA%\zakuro\settings.toml.
rem Esc opens Zakuro's menu (settings, controls), F1 pauses, F11 toggles fullscreen.
setlocal
set "HERE=%~dp0"
set "ROM=%HERE%..\Fire Emblem - Awakening (Europe) (En,Fr,De,Es,It).cci"
set "EXE=%HERE%FireEmblemAwakeningRecomp\target\release\fe-awakening.exe"
if not exist "%EXE%" (
    echo The port is not built yet. See README.md, "Rebuilding".
    pause
    exit /b 1
)
if not exist "%ROM%" (
    echo Could not find the game at "%ROM%".
    pause
    exit /b 1
)
"%EXE%" "%ROM%" --data "%HERE%userdata" %*
