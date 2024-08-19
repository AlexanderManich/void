use std::ptr::null_mut;

use crate::{
    game::{push_size_, MemoryArena},
    macros::push_array,
    tile,
};

pub struct TileMap {
    pub chunk_shift: u32,
    pub chunk_mask: u32,
    pub chunk_dim: u32,

    pub tile_side_in_meters: f32,

    pub tile_chunk_count_x: u32,
    pub tile_chunk_count_y: u32,
    pub tile_chunk_count_z: u32,

    pub tile_chunks: *mut TileChunk,
}

impl Default for TileMap {
    fn default() -> Self {
        Self {
            chunk_shift: 0,
            chunk_mask: 0,
            chunk_dim: 0,

            tile_side_in_meters: 0.0,
            tile_chunk_count_x: 0,
            tile_chunk_count_y: 0,
            tile_chunk_count_z: 0,

            tile_chunks: null_mut(),
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct TileMapPosition {
    // High bits are the tile chunk index, and the low bits are the tile index in the chunk
    pub abs_tile_x: u32,
    pub abs_tile_y: u32,
    pub abs_tile_z: u32,

    // Offsets from tile center
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TileChunk {
    pub tiles: *mut u32,
}

impl Default for TileChunk {
    fn default() -> Self {
        Self { tiles: null_mut() }
    }
}

#[derive(Default)]
pub struct TileChunkPosition {
    tile_chunk_x: u32,
    tile_chunk_y: u32,
    tile_chunk_z: u32,

    rel_tile_x: u32,
    rel_tile_y: u32,
}

fn get_tile_chunk(
    tile_map: &TileMap,
    tile_chunk_x: u32,
    tile_chunk_y: u32,
    tile_chunk_z: u32,
) -> *mut TileChunk {
    let mut result = null_mut();

    if tile_chunk_x >= 0
        && tile_chunk_x < tile_map.tile_chunk_count_x
        && tile_chunk_y >= 0
        && tile_chunk_y < tile_map.tile_chunk_count_y
        && tile_chunk_z >= 0
        && tile_chunk_z < tile_map.tile_chunk_count_z
    {
        result = unsafe {
            tile_map.tile_chunks.add(
                (tile_chunk_z * tile_map.tile_chunk_count_y * tile_map.tile_chunk_count_x
                    + tile_chunk_y * tile_map.tile_chunk_count_x
                    + tile_chunk_x) as usize,
            )
        };
    };
    result
}

pub fn get_tile_value(
    tile_map: &TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
) -> u32 {
    let chunk_pos = get_chunk_pos_for(tile_map, abs_tile_x, abs_tile_y, abs_tile_z);
    let tile_chunk = get_tile_chunk(
        &tile_map,
        chunk_pos.tile_chunk_x,
        chunk_pos.tile_chunk_y,
        chunk_pos.tile_chunk_z,
    );
    let tile_chunk_value = _get_tile_value(
        tile_map,
        tile_chunk,
        chunk_pos.rel_tile_x,
        chunk_pos.rel_tile_y,
    );

    tile_chunk_value
}

pub fn get_tile_value_from_map_pos(tile_map: &TileMap, pos: TileMapPosition) -> u32 {
    let tile_chunk_value = get_tile_value(tile_map, pos.abs_tile_x, pos.abs_tile_y, pos.abs_tile_z);

    tile_chunk_value
}

fn _get_tile_value(
    tile_map: &TileMap,
    tile_chunk: *mut TileChunk,
    test_tile_x: u32,
    test_tile_y: u32,
) -> u32 {
    let mut tile_chunk_value = 0;

    unsafe {
        if !(tile_chunk.is_null()) && !((*tile_chunk).tiles.is_null()) {
            tile_chunk_value = get_tile_value_unchecked(
                tile_map,
                tile_chunk.as_mut().unwrap(),
                test_tile_x,
                test_tile_y,
            );
        }
    }

    tile_chunk_value
}

fn get_tile_value_unchecked(
    tile_map: &TileMap,
    tile_chunk: &mut TileChunk,
    tile_x: u32,
    tile_y: u32,
) -> u32 {
    debug_assert!(!tile_chunk.tiles.is_null());
    debug_assert!(tile_x < tile_map.chunk_dim);
    debug_assert!(tile_y < tile_map.chunk_dim);

    let tile_chunk_value = unsafe {
        *tile_chunk
            .tiles
            .add(((tile_y * tile_map.chunk_dim as u32) + tile_x) as usize)
    };
    tile_chunk_value
}

fn get_chunk_pos_for(
    tile_map: &TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
) -> TileChunkPosition {
    let mut result = TileChunkPosition::default();

    result.tile_chunk_x = abs_tile_x >> tile_map.chunk_shift;
    result.tile_chunk_y = abs_tile_y >> tile_map.chunk_shift;
    result.tile_chunk_z = abs_tile_z;
    result.rel_tile_x = abs_tile_x & tile_map.chunk_mask;
    result.rel_tile_y = abs_tile_y & tile_map.chunk_mask;

    result
}

pub fn is_tile_map_point_empty(tile_map: &TileMap, can_pos: TileMapPosition) -> bool {
    let tile_chunk_value = get_tile_value(
        tile_map,
        can_pos.abs_tile_x,
        can_pos.abs_tile_y,
        can_pos.abs_tile_z,
    );
    let is_empty = tile_chunk_value == 1 || tile_chunk_value == 3 || tile_chunk_value == 4;

    is_empty
}

fn recanonicalize_coord(tile_map: &TileMap, tile: &mut u32, tile_rel: &mut f32) {
    let offset: i32 = ((*tile_rel) / tile_map.tile_side_in_meters as f32).round() as i32;
    (*tile) = ((*tile) as i32 + offset) as u32;
    (*tile_rel) = (*tile_rel) - offset as f32 * tile_map.tile_side_in_meters;

    debug_assert!((*tile_rel) >= -0.5 * tile_map.tile_side_in_meters);
    debug_assert!((*tile_rel) <= 0.5 * tile_map.tile_side_in_meters as f32);
}

pub fn recanonicalize_position(tile_map: &TileMap, pos: TileMapPosition) -> TileMapPosition {
    let mut result = pos;
    recanonicalize_coord(tile_map, &mut result.abs_tile_x, &mut result.offset_x);
    recanonicalize_coord(tile_map, &mut result.abs_tile_y, &mut result.offset_y);
    result
}

pub fn set_tile_value(
    arena: &mut MemoryArena,
    tile_map: &mut TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
    tile_value: u32,
) {
    let chunk_pos = get_chunk_pos_for(tile_map, abs_tile_x, abs_tile_y, abs_tile_z);

    let tile_chunk = get_tile_chunk(
        tile_map,
        chunk_pos.tile_chunk_x,
        chunk_pos.tile_chunk_y,
        chunk_pos.tile_chunk_z,
    );

    debug_assert!(!tile_chunk.is_null());

    let tile_chunk = unsafe { tile_chunk.as_mut().unwrap() };

    if tile_chunk.tiles.is_null() {
        let tile_count = tile_map.chunk_dim * tile_map.chunk_dim;

        tile_chunk.tiles = push_array!(arena, tile_count, u32);

        for tile_index in 0..tile_count {
            unsafe { *tile_chunk.tiles.add(tile_index as usize) = 1 };
        }
    }
    _set_tile_value(
        tile_map,
        tile_chunk,
        chunk_pos.rel_tile_x,
        chunk_pos.rel_tile_y,
        tile_value,
    );
}

fn _set_tile_value(
    tile_map: &TileMap,
    tile_chunk: &mut TileChunk,
    test_tile_x: u32,
    test_tile_y: u32,
    tile_value: u32,
) {
    if !tile_chunk.tiles.is_null() {
        set_tile_value_unchecked(tile_map, tile_chunk, test_tile_x, test_tile_y, tile_value);
    }
}

fn set_tile_value_unchecked(
    tile_map: &TileMap,
    tile_chunk: &mut TileChunk,
    tile_x: u32,
    tile_y: u32,
    tile_value: u32,
) {
    debug_assert!(!tile_chunk.tiles.is_null());
    debug_assert!(tile_x < tile_map.chunk_dim);
    debug_assert!(tile_y < tile_map.chunk_dim);

    unsafe {
        *tile_chunk
            .tiles
            .add(((tile_y * tile_map.chunk_dim as u32) + tile_x) as usize) = tile_value;
    }
}

pub fn are_on_same_tile(a: &TileMapPosition, b: &TileMapPosition) -> bool {
    let result = a.abs_tile_x == b.abs_tile_x
        && a.abs_tile_y == b.abs_tile_y
        && a.abs_tile_z == b.abs_tile_z;

    result
}
