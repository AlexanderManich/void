@echo off

set CFLAGS=-Wall -Wextra -pedantic -g
set INCLUDE=-Igame -Ilib
set IMPL=platform\win32_platform.cpp game\game.cpp lib\lib_string.c lib\lib_arena.c lib\lib_base.c
set LIBS=-lgdi32 -lwinmm
set DFLAGS=-DINTERNAL -DSLOW

if not exist build mkdir build
g++ %CFLAGS% %INCLUDE% %DFLAGS% %IMPL% -o build\void.exe %LIBS% 