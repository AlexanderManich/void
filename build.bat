@echo off

set CFLAGS=-Wall -Wextra -g

set INCLUDE=-Igame
set LIBS=-lgdi32 -lwinmm
set DFLAGS=-DINTERNAL -DSLOW

if not exist build mkdir build
gcc -c %CFLAGS% %DFLAGS% game\game.c -o build/game.o
gcc -shared -fPIC -o build/game.dll build/game.o
gcc %CFLAGS% %DFLAGS% platform\win32_platform.c %INCLUDE% %LIBS% -o build\void.exe