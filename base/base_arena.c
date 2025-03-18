static Arena *arena_init(Arena *arena, u8 *base, u64 size) {
    arena->base = base;
    arena->size = size;
    arena->pos = 0;
    return arena;
}

void *arena_push(Arena *arena, u64 size, u64 align) {
    u64 pos_pre  = align_pow2(arena->pos, align);
    u64 pos_post = pos_pre + size;
    
    assert(pos_post <= arena->size && "Arena ran out of space");

    void *result = arena->base + pos_pre;
    arena->pos = pos_post;

    return result;
}

void arena_pop_to(Arena* arena, u64 pos) {
    assert(pos <= arena->size);
    arena->pos = pos;
}

u64 arena_pos(Arena *arena) {
    u64 pos = arena->pos;
    return pos;
}

void arena_clear(Arena *arena) {
    arena->pos = 0;
}

TempArena temp_begin(Arena *arena) {
    u64 pos = arena_pos(arena);
    TempArena temp = {arena, pos};
    return temp;
}

void temp_end(TempArena temp) {
    arena_pop_to(temp.arena, temp.pos);
}