// chunk_column.rs

use crate::chunk::Chunk;
use crate::column_pos::ColumnPos;
use crate::sorted_vec::SortedVec;

pub struct ChunkColumn {
    pub pos: ColumnPos,
    chunks: SortedVec<Chunk, fn(&Chunk) -> i32>
}

impl ChunkColumn {
    pub fn new(pos: ColumnPos) -> Self {
        Self {
            pos,
            chunks: SortedVec::new(|chunk| chunk.pos.y)
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn get_chunk(&self, y: i32) -> Option<&Chunk> {
        self.chunks.get(y)
    }    
    
    pub fn get_chunk_mut(&mut self, y: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(y)
    }

    pub fn put_chunk(&mut self, chunk: Chunk) {
        self.chunks.put(chunk);
    }

    pub fn remove_chunk_at(&mut self, y: i32) -> Option<Chunk> {
        self.chunks.remove_by_key(y)
    }

    pub fn remove_chunk(&mut self, chunk: &Chunk) -> Option<Chunk> {
        self.chunks.remove_by_key(chunk.pos.y)
    }
    
    pub fn iter(&self) -> std::slice::Iter<Chunk> {
        self.chunks.iter()
    }
    
    pub fn iter_mut(&mut self) -> std::slice::IterMut<Chunk> {
        self.chunks.iter_mut()
    }
    
}