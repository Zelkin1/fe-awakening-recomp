@echo off
rem Fire Emblem: Awakening on one 16:9 screen: the top screen alone, its 3D
rem drawn wider by the game's 16:9 enhancement. F10 flips to the bottom
rem screen and back, F9 goes round the other layouts.
rem The layout and shape are for this run, and stay in Zakuro's settings
rem only once something in the game saves them, F10 does.
call "%~dp0Play-FireEmblemAwakening.bat" --layout top --top-shape 16:9 --enhance 16:9 %*
