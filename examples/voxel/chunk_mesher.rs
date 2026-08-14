//! chunk_mesher.rs

use crate::chunk::Chunk;
use crate::chunk_cache::ChunkCache;
use crate::ChunkVertex;
use crate::level::Level;

pub struct ChunkMesher;

impl ChunkMesher {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_mesh_vertices(&mut self, level: &Level, chunk: &Chunk) -> (Vec<ChunkVertex>, Vec<u32>) {
        let mut cache = ChunkCache::new();
        cache.update(level, &chunk.pos);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Данные граней: (смещение проверки, вершины, координаты соседей для AO для каждого из 4 углов)
        // Каждая вершина имеет 3 смещения для AO: [side1, side2, corner]
        let faces = [
            (
                [0, 0, 1], // Front
                [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]],
                [
                    [[-1, 0, 1], [0, -1, 1], [-1, -1, 1]],
                    [[ 1, 0, 1], [0, -1, 1], [ 1, -1, 1]],
                    [[ 1, 0, 1], [0,  1, 1], [ 1,  1, 1]],
                    [[-1, 0, 1], [0,  1, 1], [-1,  1, 1]],
                ]
            ),
            (
                [0, 0, -1], // Back
                [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
                [
                    [[ 1, 0, -1], [0, -1, -1], [ 1, -1, -1]],
                    [[-1, 0, -1], [0, -1, -1], [-1, -1, -1]],
                    [[-1, 0, -1], [0,  1, -1], [-1,  1, -1]],
                    [[ 1, 0, -1], [0,  1, -1], [ 1,  1, -1]],
                ]
            ),
            (
                [-1, 0, 0], // Left
                [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]],
                [
                    [[-1, 0, -1], [-1, -1, 0], [-1, -1, -1]],
                    [[-1, 0,  1], [-1, -1, 0], [-1, -1,  1]],
                    [[-1, 0,  1], [-1,  1, 0], [-1,  1,  1]],
                    [[-1, 0, -1], [-1,  1, 0], [-1,  1, -1]],
                ]
            ),
            (
                [1, 0, 0], // Right
                [[1.0, 0.0, 1.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]],
                [
                    [[1, 0,  1], [1, -1, 0], [1, -1,  1]],
                    [[1, 0, -1], [1, -1, 0], [1, -1, -1]],
                    [[1, 0, -1], [1,  1, 0], [1,  1, -1]],
                    [[1, 0,  1], [1,  1, 0], [1,  1,  1]],
                ]
            ),
            (
                [0, 1, 0], // Top
                [[0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
                [
                    [[-1, 1, 0], [0, 1,  1], [-1, 1,  1]],
                    [[ 1, 1, 0], [0, 1,  1], [ 1, 1,  1]],
                    [[ 1, 1, 0], [0, 1, -1], [ 1, 1, -1]],
                    [[-1, 1, 0], [0, 1, -1], [-1, 1, -1]],
                ]
            ),
            (
                [0, -1, 0], // Bottom
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
                [
                    [[-1, -1, 0], [0, -1, -1], [-1, -1, -1]],
                    [[ 1, -1, 0], [0, -1, -1], [ 1, -1, -1]],
                    [[ 1, -1, 0], [0, -1,  1], [ 1, -1,  1]],
                    [[-1, -1, 0], [0, -1,  1], [-1, -1,  1]],
                ]
            ),
        ];

        let uv_coords = [
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ];

        for x in 0..crate::chunk::SIZE {
            for y in 0..crate::chunk::SIZE {
                for z in 0..crate::chunk::SIZE {
                    let block_type = chunk.blocks.get_or_default(x, y, z, 0);
                    if block_type == 0 { continue; }

                    let texture_id = (block_type as i32 - 1) as u32;

                    for (dir, corners, ao_neighbors) in &faces {
                        let nx = x as i32 + dir[0];
                        let ny = y as i32 + dir[1];
                        let nz = z as i32 + dir[2];

                        if !self.is_solid(&cache, nx, ny, nz) {
                            let start_idx = vertices.len() as u32;
                            let mut aos = [1.0f32; 4];

                            for i in 0..4 {
                                let s1 = self.is_solid(&cache, x as i32 + ao_neighbors[i][0][0], y as i32 + ao_neighbors[i][0][1], z as i32 + ao_neighbors[i][0][2]);
                                let s2 = self.is_solid(&cache, x as i32 + ao_neighbors[i][1][0], y as i32 + ao_neighbors[i][1][1], z as i32 + ao_neighbors[i][1][2]);
                                let c  = self.is_solid(&cache, x as i32 + ao_neighbors[i][2][0], y as i32 + ao_neighbors[i][2][1], z as i32 + ao_neighbors[i][2][2]);

                                aos[i] = calc_ao_factor(s1, s2, c);

                                vertices.push(ChunkVertex {
                                    position: [
                                        x as f32 + corners[i][0],
                                        y as f32 + corners[i][1],
                                        z as f32 + corners[i][2],
                                    ],
                                    uv: uv_coords[i],
                                    texture_id,
                                    shade: aos[i],
                                });
                            }

                            if aos[0] + aos[2] > aos[1] + aos[3] {
                                indices.extend_from_slice(&[
                                    start_idx + 0, start_idx + 1, start_idx + 2,
                                    start_idx + 2, start_idx + 3, start_idx + 0,
                                ]);
                            } else {
                                indices.extend_from_slice(&[
                                    start_idx + 1, start_idx + 2, start_idx + 3,
                                    start_idx + 3, start_idx + 0, start_idx + 1,
                                ]);
                            }
                        }
                    }
                }
            }
        }

        (vertices, indices)
    }

    // Вспомогательная функция проверки непрозрачности блока
    fn is_solid(&self, cache: &ChunkCache, x: i32, y: i32, z: i32) -> bool {
        cache.get_block(x, y, z).unwrap_or(0) != 0
    }
}


// Формула рассчета интенсивности света для угла
fn calc_ao_factor(side1: bool, side2: bool, corner: bool) -> f32 {
    if side1 && side2 {
        return 0.2; // Если угол закрыт с двух сторон — максимальная тень
    }

    let count = side1 as u32 + side2 as u32 + corner as u32;
    match count {
        0 => 1.0,  // Нет препятствий — полный свет
        1 => 0.7,  // Одно препятствие
        2 => 0.4,  // Два препятствия
        _ => 0.2,  // Полностью угловая тень
    }
}