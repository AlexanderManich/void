#pragma once

typedef struct TileChunk TileChunk;
struct TileChunk {
    u32 *tiles;
};

typedef struct TileChunkPosition TileChunkPosition;
struct TileChunkPosition {
    u32 tile_chunk_x;
    u32 tile_chunk_y;
    u32 tile_chunk_z;

    u32 rel_tile_x;
    u32 rel_tile_y;
};

typedef struct TileMap TileMap;
struct TileMap {
    u32 chunk_shift;
    u32 chunk_mask;
    u32 chunk_dim;

    f32 tile_side_in_meters;

    u32 tile_chunk_count_x;
    u32 tile_chunk_count_y;
    u32 tile_chunk_count_z;

    TileChunk *tile_chunks;
};

typedef struct TileMapPosition TileMapPosition;
struct TileMapPosition {
    u32 abs_tile_x;
    u32 abs_tile_y;
    u32 abs_tile_z;

    Vec2 offset;
};

TileChunkPosition get_chunk_pos_for(TileMap *tile_map, u32 abs_tile_x, u32 abs_tile_y, u32 abs_tile_z);
TileChunk *get_tile_chunk(TileMap *tile_map, u32 tile_chunk_x, u32 tile_chunk_y, u32 tile_chunk_z);

void set_tile_value_unchecked(TileMap *tile_map, TileChunk *tile_chunk, u32 tile_x, u32 tile_y, u32 tile_value);
void _set_tile_value(TileMap *tile_map, TileChunk *tile_chunk, u32 test_tile_x, u32 test_tile_y, u32 tile_value);
void set_tile_value(Arena *arena, TileMap *tile_map, u32 abs_tile_x, u32 abs_tile_y, u32 abs_tile_z, u32 tile_value);

u32 get_tile_value_unchecked(TileMap *tile_map, TileChunk *tile_chunk, u32 tile_x, u32 tile_y);
u32 _get_tile_value(TileMap *tile_map, TileChunk *tile_chunk, u32 test_tile_x, u32 test_tile_y);
u32 get_tile_value(TileMap *tile_map, u32 abs_tile_x, u32 abs_tile_y, u32 abs_tile_z);
u32 get_tile_value_from_map_pos(TileMap *tile_map, TileMapPosition pos);

void recanonicalize_coord(TileMap *tile_map, u32 *tile, f32 *tile_rel);
TileMapPosition recanonicalize_position(TileMap *tile_map, TileMapPosition pos);

b32 is_tile_map_point_empty(TileMap *tile_map, TileMapPosition can_pos);
b32 are_on_same_tile(TileMapPosition *a, TileMapPosition *b);