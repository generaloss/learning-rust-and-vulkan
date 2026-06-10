use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet, allocator::StandardDescriptorSetAllocator};
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::Pipeline;
use crate::engine::vulkan_context::VulkanContext;
use crate::engine::shader::Shader;
use crate::engine::texture::Texture;

// Модули шейдеров теперь изолированы внутри самого бетча
mod v_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/sprite_batch.vert",
    }
}

mod f_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/sprite_batch.frag",
    }
}

// Структура вершины инкапсулирована внутри компонента отрисовки
#[derive(BufferContents, Vertex, Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SbVertex {
    #[format(R32G32_SFLOAT)]
    pub a_pos: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub a_uv: [f32; 2],
    #[format(R32_UINT)]
    pub a_texture_id: u32,
}

pub struct SpriteBatch {
    vertices: Vec<SbVertex>,
    vertex_buffer: Subbuffer<[SbVertex]>,
    max_vertices: usize,

    // Внутренние "корни" Vulkan, скрытые от внешнего кода
    shader: Shader,
    ds_allocator: Arc<StandardDescriptorSetAllocator>,
    global_descriptor_set: Option<Arc<DescriptorSet>>,
}

impl SpriteBatch {
    pub fn new(vulkan_context: &VulkanContext, max_quads: usize) -> Self {
        let max_vertices = max_quads * 6;
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(vulkan_context.device.clone()));

        // Создаем внутренний вершинный буфер
        let vertex_buffer = Buffer::new_slice::<SbVertex>(
            memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            max_vertices as u64,
        ).unwrap();

        // Загружаем и компилируем шейдеры прямо на месте
        let vs = v_shader::load(vulkan_context.device.clone()).unwrap();
        let fs = f_shader::load(vulkan_context.device.clone()).unwrap();
        let vs_entry = vs.entry_point("main").unwrap();
        let fs_entry = fs.entry_point("main").unwrap();

        // Извлекаем описание формата вершин автоматически из локальной структуры
        let vertex_definition = SbVertex::per_vertex().definition(&vs_entry).unwrap();

        // Инициализируем объект Shader локально
        let shader = Shader::new(vulkan_context, vs_entry, fs_entry, vertex_definition);

        // Создаем аллокатор дескрипторов, который теперь принадлежит бетчу
        let ds_allocator = Arc::new(StandardDescriptorSetAllocator::new(vulkan_context.device.clone(), Default::default()));

        Self {
            vertices: Vec::with_capacity(max_vertices),
            vertex_buffer,
            max_vertices,
            shader,
            ds_allocator,
            global_descriptor_set: None,
        }
    }

    // Публичный метод для конфигурации текстурного пула
    pub fn set_textures(&mut self, textures: &[&Texture]) {
        if textures.is_empty() {
            return;
        }

        let ds_layout = self.shader.layout().set_layouts().get(0).unwrap().clone();
        let mut texture_bindings = Vec::new();

        // Переносим ссылки на реальные текстуры в массив привязок
        for t in textures {
            texture_bindings.push((t.image_view.clone(), t.sampler.clone()));
        }

        // Заполняем оставшиеся пустые ячейки (до 32 штук) первой доступной текстурой
        while texture_bindings.len() < 32 {
            texture_bindings.push((textures[0].image_view.clone(), textures[0].sampler.clone()));
        }

        // Строим Descriptor Set локально внутри класса
        let descriptor_set = DescriptorSet::new(
            self.ds_allocator.clone(),
            ds_layout,
            vec![
                WriteDescriptorSet::image_view_sampler_array(0, 0, texture_bindings)
            ],
            vec![],
        ).unwrap();

        self.global_descriptor_set = Some(descriptor_set);
    }

    pub fn begin(&mut self) {
        self.vertices.clear();
    }

    pub fn draw_quad(&mut self, x: f32, y: f32, width: f32, height: f32, texture_id: u32) {
        if self.vertices.len() + 6 > self.max_vertices {
            return;
        }

        self.vertices.push(SbVertex { a_pos: [x, y],                 a_uv: [0.0, 1.0], a_texture_id: texture_id });
        self.vertices.push(SbVertex { a_pos: [x, y + height],        a_uv: [0.0, 0.0], a_texture_id: texture_id });
        self.vertices.push(SbVertex { a_pos: [x + width, y + height], a_uv: [1.0, 0.0], a_texture_id: texture_id });

        self.vertices.push(SbVertex { a_pos: [x + width, y + height], a_uv: [1.0, 0.0], a_texture_id: texture_id });
        self.vertices.push(SbVertex { a_pos: [x + width, y],         a_uv: [1.0, 1.0], a_texture_id: texture_id });
        self.vertices.push(SbVertex { a_pos: [x, y],                 a_uv: [0.0, 1.0], a_texture_id: texture_id });
    }

    // Теперь метод end сам знает, как биндить конвейеры, буферы и дескрипторы.
    // Извне мы требуем только билдер команд и матрицу трансформации камеры.
    pub fn end(&mut self, builder: &mut vulkano::command_buffer::AutoCommandBufferBuilder<vulkano::command_buffer::PrimaryAutoCommandBuffer>, combined_matrix: [[f32; 4]; 4]) {
        if self.vertices.is_empty() {
            return;
        }

        // Защита: если текстуры не были установлены через set_textures, прерываем отрисовку
        let descriptor_set = match &self.global_descriptor_set {
            Some(ds) => ds.clone(),
            None => return,
        };

        // 1. Моментально переносим сформированные вершины на GPU
        {
            let mut write_guard = self.vertex_buffer.write().unwrap();
            write_guard[0..self.vertices.len()].copy_from_slice(&self.vertices);
        }

        // 2. Формируем структуру Push Constants локально
        let push_constants = v_shader::PushConstants {
            u_combined: combined_matrix,
        };

        // 3. Полностью биндим всё внутреннее состояние графического конвейера
        builder
            .bind_pipeline_graphics(self.shader.pipeline.clone()).unwrap()
            .push_constants(self.shader.pipeline.layout().clone(), 0, push_constants).unwrap();

        builder.bind_vertex_buffers(0, self.vertex_buffer.clone()).unwrap();

        builder.bind_descriptor_sets(
            vulkano::pipeline::PipelineBindPoint::Graphics,
            self.shader.layout().clone(),
            0,
            descriptor_set,
        ).unwrap();

        // 4. Осуществляем единый атомарный вызов отрисовки геометрии
        unsafe {
            builder.draw(self.vertices.len() as u32, 1, 0, 0).unwrap();
        }
    }
}