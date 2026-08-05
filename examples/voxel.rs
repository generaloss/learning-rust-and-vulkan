use std::sync::Arc;
use glam::Vec3;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::vertex_input::{Vertex as VulkanVertex, VertexDefinition};
use vulkano::pipeline::PipelineBindPoint;
use winit::error::EventLoopError;
use winit::window::CursorGrabMode;
// Импорты модулей вашего движка
use engine::application_context::{AppAdapter, ContextBuilder, ContextFields, ContextManager};
use engine::camera::CameraPerspective;
use engine::input::KeyCode;
use engine::shader::Shader;
use engine::texture::Texture;
use engine::vertex_buffer::VertexBuffer;

// -----------------------------------------------------------------------------
// 1. Шейдеры (GLSL)
// -----------------------------------------------------------------------------
mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r#"
            #version 450

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec2 uv;
            layout(location = 2) in uint texture_id;

            layout(location = 0) out vec2 out_uv;
            layout(location = 1) flat out uint out_texture_id;

            layout(set = 0, binding = 0) uniform CameraData {
                mat4 mvp;
            } camera;

            void main() {
                gl_Position = camera.mvp * vec4(position, 1.0);
                out_uv = uv;
                out_texture_id = texture_id;
            }
        "#
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r#"
            #version 450
            #extension GL_EXT_nonuniform_qualifier : enable

            layout(location = 0) in vec2 in_uv;
            layout(location = 1) flat in uint in_texture_id;

            layout(location = 0) out vec4 f_color;

            // Массив текстур блока
            layout(set = 0, binding = 1) uniform sampler2D textures[4];

            void main() {
                f_color = texture(textures[nonuniformEXT(in_texture_id)], in_uv);
            }
        "#
    }
}

// -----------------------------------------------------------------------------
// 2. Вершина вокселя и структура камеры для GPU
// -----------------------------------------------------------------------------
#[derive(BufferContents, VulkanVertex, Clone, Copy, Debug)]
#[repr(C)]
pub struct VoxelVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
    #[format(R32_UINT)]
    pub texture_id: u32,
}

#[derive(BufferContents, Clone, Copy, Debug)]
#[repr(C)]
pub struct CameraData {
    pub mvp: [[f32; 4]; 4],
}

// -----------------------------------------------------------------------------
// 3. Воксельный Чанк (16x16x16)
// -----------------------------------------------------------------------------
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

    // Алгоритм Face Culling
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

        let uv_coords = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

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

// -----------------------------------------------------------------------------
// 4. Главное приложение
// -----------------------------------------------------------------------------
fn main() -> Result<(), EventLoopError> {
    let mut context = ContextBuilder::new()
        .title("Voxel Renderer")
        .size(1280, 720)
        .icon("assets/icon.png")
        .create();

    context.set_app(VoxelTest::new());

    let mut manager = ContextManager::new();
    manager.register(context);
    manager.run()?;

    Ok(())
}

struct VoxelTest {
    textures: Vec<Texture>,
    camera: CameraPerspective,

    player_speed: f32,
    mouse_sensitivity: f32,

    yaw: f32,
    pitch: f32,

    chunk: Chunk,
    vertex_buffer: Option<VertexBuffer<VoxelVertex>>,
    index_buffer: Option<Subbuffer<[u32]>>,
    index_count: u32,

    shader: Option<Shader>,
    camera_buffer: Option<Subbuffer<CameraData>>,
    descriptor_set: Option<Arc<DescriptorSet>>,
}

impl VoxelTest {
    fn new() -> Self {
        Self {
            textures: Vec::new(),
            camera: CameraPerspective::new(),

            player_speed: 12.0,
            mouse_sensitivity: 0.002,

            yaw: 0.0,
            pitch: 0.0,

            chunk: Chunk::new(),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,

            shader: None,
            camera_buffer: None,
            descriptor_set: None,
        }
    }

    fn build_pipeline_and_buffers(&mut self, fields: &mut ContextFields) {
        let vulkan = &fields.vulkan;

        // 1. Создаем буферы геометрии
        let (vertices, indices) = self.chunk.generate_mesh();
        self.index_count = indices.len() as u32;
        if vertices.is_empty() { return; }

        self.vertex_buffer = Some(VertexBuffer::new(vulkan, vertices));

        self.index_buffer = Some(
            Buffer::from_iter(
                vulkan.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::INDEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                indices,
            ).unwrap()
        );

        // 2. Создаем шейдеры и пайплайн через модуль Shader движка
        let vs = vs::load(vulkan.device.clone()).unwrap();
        let fs = fs::load(vulkan.device.clone()).unwrap();

        let vs_entry = vs.entry_point("main").unwrap();
        let fs_entry = fs.entry_point("main").unwrap();

        let vertex_definition = VoxelVertex::per_vertex().definition(&vs_entry).unwrap();

        let shader = Shader::new(vulkan, vs_entry, fs_entry, vertex_definition);

        // 3. Создаем Uniform-буфер для матрицы камеры
        let camera_buffer = Buffer::from_data(
            vulkan.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            CameraData { mvp: self.camera.combined.to_cols_array_2d() },
        ).unwrap();

        // 4. Формируем DescriptorSet для связи шейдера с буфером и текстурами
        let ds_allocator = Arc::new(StandardDescriptorSetAllocator::new(vulkan.device.clone(), Default::default()));
        let ds_layout = shader.layout().set_layouts().get(0).unwrap().clone();

        let textures_and_samplers: Vec<_> = self.textures
            .iter()
            .map(|t| (t.image_view.clone(), t.sampler.clone()))
            .collect();

        let descriptor_set = DescriptorSet::new(
            ds_allocator,
            ds_layout,
            vec![
                WriteDescriptorSet::buffer(0, camera_buffer.clone()),
                WriteDescriptorSet::image_view_sampler_array(1, 0, textures_and_samplers),
            ],
            vec![],
        ).unwrap();

        self.shader = Some(shader);
        self.camera_buffer = Some(camera_buffer);
        self.descriptor_set = Some(descriptor_set);
    }
}

impl AppAdapter for VoxelTest {
    fn init(&mut self, fields: &mut ContextFields) {
        // Загрузка текстур через модуль Texture
        self.textures = vec![
            Texture::from_path(&fields.vulkan, "assets/tiles/grass.png"),
            Texture::from_path(&fields.vulkan, "assets/tiles/dirt.png"),
            Texture::from_path(&fields.vulkan, "assets/tiles/stone.png"),
            Texture::from_path(&fields.vulkan, "assets/tiles/planks.png"),
        ];

        // grab & hide cursor
        fields.window.set_cursor_grab(CursorGrabMode::Locked).expect("Cannot lock cursor");
        fields.window.set_cursor_visible(false);

        // Первоначальный размер камеры под окно
        let window_size = fields.window.inner_size();
        self.camera.resize(window_size.width as f32, window_size.height as f32);
        self.camera.position = [8.0, 18.0, 24.0].into();

        self.build_pipeline_and_buffers(fields);
    }

    fn update(&mut self, fields: &mut ContextFields) {
        let input = &fields.input;
        let dt = 0.016; // При необходимости можно вычислять реальный dt

        // Одиночные нажатия
        if input.is_key_down(KeyCode::F11) {
            fields.toggle_fullscreen();
        }
        if input.is_key_down(KeyCode::Escape) {
            fields.should_close = true;
        }

        // Обзор мышью
        let (mdx, mdy) = (input.delta.x as f32, input.delta.y as f32);
        if mdx != 0.0 || mdy != 0.0 {
            self.yaw += mdx * self.mouse_sensitivity;
            self.pitch -= mdy * self.mouse_sensitivity;

            let max_pitch = 89.0f32.to_radians();
            self.pitch = self.pitch.clamp(-max_pitch, max_pitch);

            self.camera.set_euler_angles(self.yaw, self.pitch);
        }

        // Удерживание клавиш (непрерывное движение)
        let move_speed = self.player_speed * dt;
        let forward = self.camera.forward();
        let forward_xz = Vec3::new(forward.x, 0.0, forward.z);
        let right = self.camera.right();
        let right_xz = Vec3::new(right.x, 0.0, right.z);

        if input.is_key_pressed(KeyCode::KeyW) { self.camera.position += forward_xz * move_speed; }
        if input.is_key_pressed(KeyCode::KeyS) { self.camera.position -= forward_xz * move_speed; }
        if input.is_key_pressed(KeyCode::KeyD) { self.camera.position += right_xz * move_speed; }
        if input.is_key_pressed(KeyCode::KeyA) { self.camera.position -= right_xz * move_speed; }
        if input.is_key_pressed(KeyCode::Space) { self.camera.position.y += move_speed; }
        if input.is_key_pressed(KeyCode::ShiftLeft) { self.camera.position.y -= move_speed; }

        self.camera.update();

        // Обновляем матрицу вида в памяти GPU
        if let Some(ref camera_buffer) = self.camera_buffer {
            if let Ok(mut writer) = camera_buffer.write() {
                writer.mvp = self.camera.combined.to_cols_array_2d();
            }
        }
    }

    fn render(&mut self, _fields: &mut ContextFields, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if self.index_count == 0 { return; }

        if let (Some(v_buffer), Some(i_buffer), Some(shader), Some(ds)) = (
            &self.vertex_buffer,
            &self.index_buffer,
            &self.shader,
            &self.descriptor_set,
        ) {
            // Привязываем пайплайн, дескриптор сет и буферы геометрии
            builder
                .bind_pipeline_graphics(shader.pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    shader.layout().clone(),
                    0,
                    ds.clone(),
                )
                .unwrap()
                .bind_vertex_buffers(0, v_buffer.subbuffer.clone())
                .unwrap()
                .bind_index_buffer(i_buffer.clone())
                .unwrap();

            unsafe {
                builder
                    .draw_indexed(self.index_count, 1, 0, 0, 0)
                    .unwrap();
            }
        }
    }

    fn resize(&mut self, _fields: &mut ContextFields, width: u32, height: u32) {
        self.camera.resize(width as f32, height as f32);
    }

    fn shutdown(&mut self) {
        println!("Voxel Test Shutdown");
    }
}