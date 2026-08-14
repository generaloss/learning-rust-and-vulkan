//! chunk.rs

use engine::mesh::MeshIndexed;
use crate::byte_nibble_array::ByteNibbleArray3D;
use crate::chunk_pos::ChunkPos;
use crate::ChunkVertex;

pub const SIZE: usize = 16;
pub const AREA: usize = SIZE * SIZE;
pub const VOLUME: usize = AREA * SIZE;

pub struct Chunk {
    pub pos: ChunkPos,
    pub blocks: ByteNibbleArray3D,
    pub mesh: Option<MeshIndexed<ChunkVertex, u32>>
}

impl Chunk {
    pub fn new(pos: ChunkPos) -> Self {
        Self {
            pos,
            blocks: ByteNibbleArray3D::init(0),
            mesh: None,
        }
    }
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Self {
            pos: self.pos.clone(),
            blocks: ByteNibbleArray3D::from_array(self.blocks.array),
            mesh: None,
        }
    }
}