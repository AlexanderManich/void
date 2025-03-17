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

