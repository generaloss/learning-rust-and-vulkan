// chunk_pos.rs

use std::fmt::{Display, Formatter};
use crate::chunk;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            x,
            y,
            z,
        }
    }

    pub fn block_x(&self) -> i64 {
        self.x as i64 * chunk::SIZE as i64
    }

    pub fn block_y(&self) -> i64 {
        self.y as i64 * chunk::SIZE as i64
    }

    pub fn block_z(&self) -> i64 {
        self.z as i64 * chunk::SIZE as i64
    }

    pub fn neighbor(&self, dx: i32, dy: i32, dz: i32) -> ChunkPos {
        ChunkPos::new(
            self.x + dx,
            self.y + dy,
            self.z + dz
        )
    }
}

impl Display for ChunkPos {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}, {}, {}", self.x, self.y, self.z)
    }
}