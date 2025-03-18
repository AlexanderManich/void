#include "tile.h"
#include "../base/base_include.h"

TileChunkPosition get_chunk_pos_for(TileMap *tile_map, u32 abs_tile_x, u32 abs_tile_y, u32 abs_tile_z) {
    TileChunkPosition result = {};
    result.tile_chunk_x = abs_tile_x >> tile_map->chunk_shift;
    result.tile_chunk_y = abs_tile_y >> tile_map->chunk_shift;
    result.tile_chunk_z = abs_tile_z >> tile_map->chunk_shift;
    result.rel_tile_x = abs_tile_x & tile_map->chunk_mask;
    result.rel_tile_y = abs_tile_y & tile_map->chunk_mask;

    return result;
}

TileChunk *get_tile_chunk(TileMap *tile_map, u32 tile_chunk_x, u32 tile_chunk_y, u32 tile_chunk_z) {
    TileChunk *result = 0;

    if (tile_chunk_x < tile_map->tile_chunk_count_x 
        && tile_chunk_y < tile_map->tile_chunk_count_y 
        && tile_chunk_z < tile_map->tile_chunk_count_z) {
        
        result = tile_map->tile_chunks + (
            tile_chunk_z * tile_map->tile_chunk_count_y * tile_map->tile_chunk_count_x
            + tile_chunk_y * tile_map->tile_chunk_count_x
            + tile_chunk_x
        );       
    }

    return result;
}

void set_tile_value_unchecked(TileMap *tile_map, TileChunk *tile_chunk, u32 tile_x, u32 tile_y, u32 tile_value) {
    assert(tile_chunk->tiles);
    assert(tile_x < tile_map->chunk_dim);
    assert(tile_y < tile_map->chunk_dim);

    *(tile_chunk->tiles + (tile_y * tile_map->chunk_dim + tile_x)) = tile_value;
}

void _set_tile_value(TileMap *tile_map, TileChunk *tile_chunk, u32 test_tile_x, u32 test_tile_y, u32 tile_value) {
    if (tile_chunk->tiles) {
        set_tile_value_unchecked(tile_map, tile_chunk, test_tile_x, test_tile_y, tile_value);
    }
}

void set_tile_value(Arena *arena, TileMap *tile_map, u32 abs_tile_x, u32 abs_tile_y, u32 abs_tile_z, u32 tile_value) {
    TileChunkPosition chunk_pos = get_chunk_pos_for(tile_map, abs_tile_x, abs_tile_y, abs_tile_z);

    TileChunk *tile_chunk = get_tile_chunk(tile_map, chunk_pos.tile_chunk_x, chunk_pos.tile_chunk_y, chunk_pos.tile_chunk_z);

    assert(tile_chunk);

    if (tile_chunk->tiles) {
        u32 tile_count = tile_map->chunk_dim * tile_map->chunk_dim;

        tile_chunk->tiles = push_array(arena, u32, tile_count);

        for (u32 tile_index = 0; tile_index < tile_count; tile_index++) {
            *(tile_chunk->tiles + tile_index) = 1; 
        }
    }

    _set_tile_value(tile_map, tile_chunk, chunk_pos.rel_tile_x, chunk_pos.rel_tile_y, tile_value);
}

u32 get_tile_value_unchecked(TileMap *tile_map, TileChunk *tile_chunk, u32 tile_x, u32 tile_y) {
    assert(tile_chunk->tiles);
    assert(tile_x < tile_map->chunk_dim);
    assert(tile_y < tile_map->chunk_dim);

    u32 tile_chunk_value = *(tile_chunk->tiles + (tile_y * tile_map->chunk_dim + tile_x));

    return tile_chunk_value;
}

u32 _get_tile_value(TileMap *tile_map, TileChunk *tile_chunk, u32 test_tile_x, u32 test_tile_y) {
    u32 tile_chunk_value = 0;

    if (tile_chunk && tile_chunk->tiles) {
        tile_chunk_value = get_tile_value_unchecked(tile_map, tile_chunk, test_tile_x, test_tile_y);
    }
    
    return tile_chunk_value;
}

u32 get_tile_value(TileMap *tile_map, u32 abs_tile_x, u32 abs_tile_y, u32 abs_tile_z) {
    TileChunkPosition chunk_pos = get_chunk_pos_for(tile_map, abs_tile_x, abs_tile_y, abs_tile_z);
    TileChunk *tile_chunk = get_tile_chunk(tile_map, chunk_pos.tile_chunk_x, chunk_pos.tile_chunk_y, chunk_pos.tile_chunk_z);
    u32 tile_chunk_value = _get_tile_value(tile_map, tile_chunk, chunk_pos.rel_tile_x, chunk_pos.rel_tile_y);

    return tile_chunk_value;
}

u32 get_tile_value_from_map_pos(TileMap *tile_map, TileMapPosition pos) {
    u32 tile_chunk_value = get_tile_value(tile_map, pos.abs_tile_x, pos.abs_tile_y, pos.abs_tile_z);
    return tile_chunk_value;
}

void recanonicalize_coord(TileMap *tile_map, u32 *tile, f32 *tile_rel) {
    i32 offset = round_f32_to_i32(*tile_rel / tile_map->tile_side_in_meters);
    *tile = *tile + (u32)offset;
    *tile_rel -= (f32)offset * tile_map->tile_side_in_meters;

    assert(*tile_rel >= -0.5 * tile_map->tile_side_in_meters);
    assert(*tile_rel <= 0.5 * tile_map->tile_side_in_meters);
}

TileMapPosition recanonicalize_position(TileMap *tile_map, TileMapPosition pos) {
    TileMapPosition result = pos;
    recanonicalize_coord(tile_map, &result.abs_tile_x, &result.offset.x);
    recanonicalize_coord(tile_map, &result.abs_tile_y, &result.offset.y);

    return result;
}

b32 is_tile_map_point_empty(TileMap *tile_map, TileMapPosition can_pos) {
    u32 tile_chunk_value = get_tile_value(tile_map, can_pos.abs_tile_x, can_pos.abs_tile_y, can_pos.abs_tile_z);
    b32 is_empty = tile_chunk_value == 1 || tile_chunk_value == 3 || tile_chunk_value || 4;

    return is_empty;
}

b32 are_on_same_tile(TileMapPosition *a, TileMapPosition *b) {
    b32 result = a->abs_tile_x == b->abs_tile_x 
        && a->abs_tile_y == b->abs_tile_y 
        && a->abs_tile_z == b->abs_tile_z;

    return result;
}