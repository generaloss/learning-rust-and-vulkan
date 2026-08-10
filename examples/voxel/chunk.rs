use crate::VoxelVertex;

const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    pub blocks: [u8; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    pub fn new() -> Self {
        let mut blocks = [0u8; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];

        // 0 = Воздух, 1 = Трава, 2 = Грязь, 3 = Камень, 4 = Доски
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let height = 6 + ((x as f32 * 0.5).sin() + (z as f32 * 0.5).cos()) as usize;

                for y in 0..CHUNK_SIZE {
                    let index = Self::block_index(x, y, z);
                    if y == height {
                        blocks[index] = 1;
                    } else if y < height && y >= height.saturating_sub(2) {
                        blocks[index] = 2;
                    } else if y < height.saturating_sub(2) {
                        blocks[index] = 3;
                    }
                }
            }
        }

        Self { blocks }
    }

    #[inline]
    fn block_index(x: usize, y: usize, z: usize) -> usize {
        x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= CHUNK_SIZE as i32 ||
            y < 0 || y >= CHUNK_SIZE as i32 ||
            z < 0 || z >= CHUNK_SIZE as i32 {
            return 0;
        }
        self.blocks[Self::block_index(x as usize, y as usize, z as usize)]
    }

    pub fn generate_mesh(&self) -> (Vec<VoxelVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let faces = [
            ([ 0,  0,  1], [ [0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0] ]), // Front
            ([ 0,  0, -1], [ [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0] ]), // Back
            ([-1,  0,  0], [ [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0] ]), // Left
            ([ 1,  0,  0], [ [1.0, 0.0, 1.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0] ]), // Right
            ([ 0,  1,  0], [ [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0] ]), // Top
            ([ 0, -1,  0], [ [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0] ]), // Bottom
        ];

        let uv_coords = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

        for x in 0..CHUNK_SIZE as i32 {
            for y in 0..CHUNK_SIZE as i32 {
                for z in 0..CHUNK_SIZE as i32 {
                    let block_type = self.get_block(x, y, z);
                    if block_type == 0 { continue; }

                    let texture_id = (block_type - 1) as u32;

                    for (dir, corners) in &faces {
                        let nx = x + dir[0];
                        let ny = y + dir[1];
                        let nz = z + dir[2];

                        if self.get_block(nx, ny, nz) == 0 {
                            let start_idx = vertices.len() as u32;

                            for i in 0..4 {
                                vertices.push(VoxelVertex {
                                    position: [
                                        x as f32 + corners[i][0],
                                        y as f32 + corners[i][1],
                                        z as f32 + corners[i][2],
                                    ],
                                    uv: uv_coords[i],
                                    texture_id,
                                });
                            }

                            indices.extend_from_slice(&[
                                start_idx, start_idx + 1, start_idx + 2,
                                start_idx + 2, start_idx + 3, start_idx
                            ]);
                        }
                    }
                }
            }
        }

        (vertices, indices)
    }
}