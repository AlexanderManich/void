@echo off

set CFLAGS=-Wall -Wextra -pedantic -g

set LIBS=-lgdi32 -lwinmm
set DFLAGS=-DINTERNAL -DSLOW

if not exist build mkdir build
g++ -c %CFLAGS% %DFLAGS% game\game.cpp -o build/game.o
g++ -shared -o build/game.dll build/game.o
g++ %CFLAGS% %DFLAGS% -Igame platform\win32_platform.cpp %LIBS% -o build\void.exe