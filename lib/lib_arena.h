#ifndef ARENA_H
#define ARENA_H

#include "lib_base.h"
#include "string.h"

typedef struct Arena Arena;
struct Arena {
    u8 *base;
    u64 pos;
    u64 size;
};

typedef struct TempArena TempArena;
struct TempArena {
    Arena *arena;
    u64 pos;
};

Arena *arena_init(Arena *arena, u8 *base, u64 size);
void arena_release(Arena *arena);

void *arena_push(Arena *arena, u64 size, u64 align);
void *arena_push_zero(Arena *arena, u64 size);

#define ARENA_DEFAULT_ALIGN 16
#define push_array(arena, type, count) (type *)memory_zero(push_array_no_zero(arena, type, count), sizeof(type) * (count))
#define push_array_no_zero(arena, type, count) (type *)arena_push((arena), sizeof(type)*(count), (ARENA_DEFAULT_ALIGN))

void ArenaPop(Arena *arena, u64 size);
void arena_pop_to(Arena *arena, u64 pos);
void arena_clear(Arena *arena);

u64 arena_pos(Arena *arena);

TempArena temp_begin(Arena *arena);
void temp_end(TempArena temp);

#endif //ARENA_H