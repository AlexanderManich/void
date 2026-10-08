@echo off

set CFLAGS=-Wall -Wextra -g

set INCLUDE=-Igame
set LIBS=-lgdi32 -lwinmm
set DFLAGS=-DINTERNAL -DSLOW

if not exist build mkdir build
g++ -c %CFLAGS% %DFLAGS% game\game.cpp -o build/game.o
g++ -shared -fPIC -o build/game.dll build/game.o game.def
g++ %CFLAGS% %DFLAGS% platform\win32_platform.cpp %INCLUDE% %LIBS% -o build\void.exe