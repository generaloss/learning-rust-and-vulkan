//! main.rs

pub mod chunk;
pub mod chunk_mesher;
pub mod byte_nibble_array;
pub mod level;
pub mod chunk_column;
pub mod sorted_vec;
pub mod chunk_pos;
pub mod column_pos;
pub mod chunk_cache;

use std::sync::Arc;
use glam::Vec3;
use noise::{NoiseFn, Perlin};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::vertex_input::{Vertex as VulkanVertex, VertexDefinition};
use vulkano::pipeline::PipelineBindPoint;
use winit::error::EventLoopError;
use winit::window::CursorGrabMode;

use engine::application_context::{AppAdapter, ContextBuilder, ContextFields, ContextManager};
use engine::camera::CameraPerspective;
use engine::input::KeyCode;
use engine::mesh::MeshIndexed;
use engine::shader::Shader;
use engine::texture::Texture;
use crate::chunk::{Chunk, SIZE};
use crate::chunk_mesher::ChunkMesher;
use crate::chunk_pos::ChunkPos;
use crate::level::Level;

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r#"
            #version 450

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec2 uv;
            layout(location = 2) in uint texture_id;
            layout(location = 3) in float shade;

            layout(location = 0) out vec2 out_uv;
            layout(location = 1) flat out uint out_texture_id;
            layout(location = 2) out float out_shade;

            layout(set = 0, binding = 0) uniform CameraData {
                mat4 mvp;
            } camera;

            layout(push_constant) uniform PushData {
                vec3 chunk_offset;
            } push;

            void main() {
                vec3 world_pos = position + push.chunk_offset;

                gl_Position = camera.mvp * vec4(world_pos, 1.0);
                out_uv = uv;
                out_texture_id = texture_id;
                out_shade = shade;
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
            layout(location = 2) in float in_shade;

            layout(location = 0) out vec4 f_color;

            // Массив текстур блока
            layout(set = 0, binding = 1) uniform sampler2D textures[4];

            void main() {
                vec4 tex_color = texture(textures[nonuniformEXT(in_texture_id)], in_uv);
                f_color = vec4(tex_color.rgb * in_shade, tex_color.a) * vec4(0.7, 0.7, 0.7, 1.0);
            }
        "#
    }
}

#[derive(BufferContents, VulkanVertex, Clone, Copy, Debug)]
#[repr(C)]
pub struct ChunkVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
    #[format(R32_UINT)]
    pub texture_id: u32,
    #[format(R32_SFLOAT)]
    pub shade: f32,
}

#[derive(BufferContents, Clone, Copy, Debug)]
#[repr(C)]
pub struct CameraData {
    pub mvp: [[f32; 4]; 4],
}

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

    level: Level,
    chunk_mesher: ChunkMesher,

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
            mouse_sensitivity: 0.001,

            yaw: 0.0,
            pitch: 0.0,

            level: Level::new(),
            chunk_mesher: ChunkMesher::new(),

            shader: None,
            camera_buffer: None,
            descriptor_set: None,
        }
    }

    fn build_pipeline_and_buffers(&mut self, fields: &mut ContextFields) {
        let vulkan = &fields.vulkan;

        // 2. Создаем шейдеры и пайплайн через модуль Shader движка
        let vs = vs::load(vulkan.device.clone()).unwrap();
        let fs = fs::load(vulkan.device.clone()).unwrap();

        let vs_entry = vs.entry_point("main").unwrap();
        let fs_entry = fs.entry_point("main").unwrap();

        let vertex_definition = ChunkVertex::per_vertex().definition(&vs_entry).unwrap();

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

    fn create_level(&mut self) {
        // Инициализируем генератор шума (можно передать любой u32 в качестве сида)
        let perlin = Perlin::new(1337);

        // Масштаб (частота) шума: чем меньше, тем плавнее холмы
        let scale = 0.03;
        let base_height = 12.0; // Средняя высота ландшафта
        let amplitude = 8.0;   // Максимальное отклонение высоты от средней

        // Сетка 10x2x10 чанков
        for cx in 0..10 {
            for cy in 0..2 {
                for cz in 0..10 {
                    let mut chunk = Chunk::new(ChunkPos::new(cx as i32, cy as i32, cz as i32));

                    for x in 0..SIZE {
                        for z in 0..SIZE {
                            // Вычисляем глобальные координаты блока по горизонтали
                            let world_x = (cx * SIZE + x) as f64;
                            let world_z = (cz * SIZE + z) as f64;

                            // noise_val вернёт значение в диапазоне [-1.0, 1.0]
                            let noise_val = perlin.get([world_x * scale, world_z * scale]);

                            // Вычисляем итоговую высоту ландшафта в блоках
                            let height = (base_height + noise_val * amplitude) as usize;

                            for y in 0..SIZE {
                                // Глобальная высота Y текущего блока
                                let world_y = cy * SIZE + y;

                                if world_y == height {
                                    chunk.blocks.set(x, y, z, 1); // Трава
                                } else if world_y < height && world_y >= height.saturating_sub(2) {
                                    chunk.blocks.set(x, y, z, 2); // Грязь
                                } else if world_y < height.saturating_sub(2) {
                                    chunk.blocks.set(x, y, z, 3); // Камень
                                }
                            }
                        }
                    }

                    self.level.put_chunk(chunk);
                }
            }
        }
    }

    fn build_chunk_meshes(&mut self, fields: &mut ContextFields) {
        let vulkan = &fields.vulkan;

        let mut generated_meshes = Vec::new();

        self.level.for_each_chunk(|chunk| {
            let (vertices, indices) = self.chunk_mesher.generate_mesh_vertices(&self.level, chunk);

            if !vertices.is_empty() {
                let mesh = MeshIndexed::from_data(vulkan, vertices, indices);
                generated_meshes.push((chunk.pos, mesh));
            }
        });

        for (pos, mesh) in generated_meshes {
            if let Some(chunk) = self.level.get_chunk_mut(pos.x, pos.y, pos.z) {
                chunk.mesh = Some(mesh);
            }
        }
    }
}

impl AppAdapter for VoxelTest {
    fn init(&mut self, fields: &mut ContextFields) {
        // Загрузка текстур через модуль Texture
        self.textures = vec![
            Texture::from_path(&fields.vulkan, "assets/voxel/grass_top.png"),
            Texture::from_path(&fields.vulkan, "assets/voxel/dirt.png"),
            Texture::from_path(&fields.vulkan, "assets/voxel/stone.png"),
            Texture::from_path(&fields.vulkan, "assets/voxel/planks.png"),
        ];

        // grab & hide cursor
        fields.window.set_cursor_grab(CursorGrabMode::Locked).expect("Cannot lock cursor");
        fields.window.set_cursor_visible(false);

        // Первоначальный размер камеры под окно
        let window_size = fields.window.inner_size();
        self.camera.resize(window_size.width as f32, window_size.height as f32);
        self.camera.position = [8.0, 18.0, 24.0].into();

        self.create_level();
        self.build_chunk_meshes(fields);
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
            self.yaw -= mdx * self.mouse_sensitivity;
            self.pitch -= mdy * self.mouse_sensitivity;

            let max_pitch = 89.0f32.to_radians();
            self.pitch = self.pitch.clamp(-max_pitch, max_pitch);

            self.camera.set_euler_angles(self.yaw, self.pitch);
        }

        // Удерживание клавиш (непрерывное движение)
        let move_speed = self.player_speed * dt;
        let forward = self.camera.forward();
        let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let right = self.camera.right();
        let right_xz = Vec3::new(right.x, 0.0, right.z).normalize();

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
        if let (Some(shader), Some(ds)) = (&self.shader, &self.descriptor_set) {
            // 1. Привязываем тяжёлое состояние (шейдер/пайплайн и текстуры с камерой) ОДИН раз
            builder
                .bind_pipeline_graphics(shader.pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    shader.layout().clone(),
                    0,
                    ds.clone(),
                )
                .unwrap();

            // 2. Итерируемся по всем чанкам в векторе
            self.level.for_each_chunk_mut(|chunk| {
                if let Some(mesh) = &chunk.mesh {
                    let index_count = mesh.index_buffer.len();

                    // Пропускаем пустые меши (например, полностью воздушные чанки)
                    if index_count == 0 {
                        return;
                    }

                    let push_data = vs::PushData {
                        chunk_offset: [
                            chunk.pos.block_x() as f32,
                            chunk.pos.block_y() as f32,
                            chunk.pos.block_z() as f32,
                        ]
                    };

                    // 3. Быстро перепривязываем только геометрию текущего чанка
                    builder
                        .push_constants(shader.layout().clone(), 0, push_data)
                        .unwrap()
                        .bind_vertex_buffers(0, mesh.vertex_buffer.clone().into_subbuffer())
                        .unwrap()
                        .bind_index_buffer(mesh.index_buffer.clone().into_subbuffer())
                        .unwrap();

                    // 4. Отрисовываем чанк
                    unsafe {
                        builder
                            .draw_indexed(index_count, 1, 0, 0, 0)
                            .unwrap();
                    }
                }
            });
        }
    }

    fn resize(&mut self, _fields: &mut ContextFields, width: u32, height: u32) {
        self.camera.resize(width as f32, height as f32);
    }

    fn shutdown(&mut self) {
        println!("Voxel Test Shutdown");
    }
}