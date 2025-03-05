@echo off

set CFLAGS=-Wall -Wextra -pedantic -g
set INCLUDE=-Igame
set LIBS=-lgdi32

if not exist build mkdir build
g++ %CFLAGS% %INCLUDE% platform\win32_platform.cpp game\void.cpp -o build\void.exe %LIBS% -std=c++20