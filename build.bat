@echo off

set CFLAGS=-Wall -Wextra -pedantic -g
set INCLUDE=-Igame
set LIBS=-lgdi32
set DFLAGS=-DINTERNAL -DSLOW

if not exist build mkdir build
g++ %CFLAGS% %INCLUDE% %DFLAGS% platform\win32_platform.cpp game\void.cpp -o build\void.exe %LIBS% 