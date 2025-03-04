@echo off

set CFLAGS=-Wall -Wextra -pedantic

if not exist build mkdir build
g++ %CFLAGS% -g platform\main.cpp -o build\void.exe