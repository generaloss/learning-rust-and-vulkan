//! chunk_cache.rs

use crate::chunk;
use crate::chunk::Chunk;
use crate::chunk_pos::ChunkPos;
use crate::level::Level;

const SIZE: usize = 3;
const AREA: usize = SIZE * SIZE;
const VOLUME: usize = AREA * SIZE;
const HALF_SIZE: i32 = SIZE as i32 / 2;
const CENTER_CHUNK_INDEX: usize = (VOLUME - 1) / 2;

pub struct ChunkCache<'a> {
    chunks: [Option<&'a Chunk>; VOLUME]
}

impl<'a> ChunkCache<'a> {
    pub fn new() -> Self {
        Self {
            chunks: [const { None }; VOLUME]
        }
    }

    #[inline]
    pub fn index(x: i32, y: i32, z: i32) -> usize {
        (x + HALF_SIZE) as usize +
        (y + HALF_SIZE) as usize * SIZE +
        (z + HALF_SIZE) as usize * AREA
    }

    pub fn chunk(&self, x: i32, y: i32, z: i32) -> Option<&Chunk> {
        let index = Self::index(x, y, z);
        self.chunks[index]
    }

    pub fn update(&mut self, level: &'a Level, pos: &ChunkPos) {
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let index = Self::index(x, y, z);
                    let neighbor_pos = pos.neighbor(x, y, z);
                    self.chunks[index] = level.get_chunk(neighbor_pos.x, neighbor_pos.y, neighbor_pos.z);
                }
            }
        }
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<u8> {
        let chunk_size = chunk::SIZE as i32;

        let chunk_x = x.div_euclid(chunk_size);
        let chunk_y = y.div_euclid(chunk_size);
        let chunk_z = z.div_euclid(chunk_size);

        if let Some(chunk) = self.chunk(chunk_x, chunk_y, chunk_z) {
            let block_x = x.rem_euclid(chunk_size) as usize;
            let block_y = y.rem_euclid(chunk_size) as usize;
            let block_z = z.rem_euclid(chunk_size) as usize;

            chunk.blocks.get(block_x, block_y, block_z)
        } else {
            None
        }
    }

}