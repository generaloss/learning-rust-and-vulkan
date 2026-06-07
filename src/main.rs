// 'main.rs'

pub mod engine;

use crate::engine::vulkan_context::VulkanContext;

use std::sync::Arc;
use winit::error::EventLoopError;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::buffer::BufferContents;
use vulkano::pipeline::graphics::vertex_input::{Vertex as VulkanoVertex, VertexDefinition};
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage, view::ImageView};
use vulkano::format::Format;
use vulkano::pipeline::Pipeline;
use vulkano::descriptor_set::{WriteDescriptorSet, allocator::StandardDescriptorSetAllocator, DescriptorSet};
use vulkano::image::sampler::{Sampler, SamplerCreateInfo, Filter, SamplerAddressMode};
use vulkano::sync::GpuFuture;
use crate::engine::context::{ContextBuilder, ContextManager, AppAdapter};

mod v_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/shader.vert",
    }
}
mod f_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/shader.frag",
    }
}

#[derive(BufferContents, VulkanoVertex, Clone, Copy, Debug, Default)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    a_pos: [f32; 2],
    #[format(R32G32_SFLOAT)]
    a_uv: [f32; 2],
}


struct BlazingFastApp {
    pipeline: Option<Arc<GraphicsPipeline>>,
    vertex_buffer: Option<Subbuffer<[MyVertex]>>,
    _texture: Option<Arc<Image>>,
    descriptor_set: Option<Arc<DescriptorSet>>,
}

impl BlazingFastApp {
    fn new() -> Self {
        Self {
            pipeline: None,
            vertex_buffer: None,
            _texture: None,
            descriptor_set: None,
        }
    }
}


fn main() -> Result<(), EventLoopError> {
    let mut context = ContextBuilder::new()
        .title("Blazing Fast Engine")
        .size(720, 720)
        .icon("assets/icon.png")
        .create();

    context.set_app(BlazingFastApp::new());

    let mut manager = ContextManager::new();
    manager.register(context);
    manager.run()?;
    Ok(())
}


impl AppAdapter for BlazingFastApp {
    fn init(&mut self, vulkan: &mut VulkanContext) {
        println!("Game Init: Creating textured pipeline...");

        // 1. Координаты вершин треугольника с UV-маппингом
        let vertices = [
            MyVertex { a_pos: [-1.0, -1.0], a_uv: [0.0, 0.0] },
            MyVertex { a_pos: [-1.0,  1.0], a_uv: [0.0, 1.0] },
            MyVertex { a_pos: [ 1.0,  1.0], a_uv: [1.0, 1.0] },

            MyVertex { a_pos: [ 1.0,  1.0], a_uv: [1.0, 1.0] },
            MyVertex { a_pos: [ 1.0, -1.0], a_uv: [1.0, 0.0] },
            MyVertex { a_pos: [-1.0, -1.0], a_uv: [0.0, 0.0] },
        ];

        let vertex_buffer = Buffer::from_iter(
            vulkan.memory_allocator.clone(),
            BufferCreateInfo { usage: BufferUsage::VERTEX_BUFFER, ..Default::default() },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices,
        ).unwrap();

        // 2. Загружаем текстуру из файла с помощью крейта image
        // Укажите правильный путь к вашей картинке (например, в корне проекта создайте папку assets)
        let img = image::open("assets/image.jpg")
            .expect("Failed to open image file!")
            .to_rgba8(); // Конвертируем в формат RGBA (8 бит на канал)

        let (width, height) = img.dimensions();
        // Переводим пиксели в сырой вектор Vec<u8>
        let texture_data = img.into_raw();

        let texture = Image::new(
            vulkan.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM, // Формат идеально совпадает с .to_rgba8()
                extent: [width, height, 1],     // Подставляем динамические размеры картинки
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            }
        ).unwrap();

        // Загружаем пиксели в текстуру (простейший способ через upload_from_iterator)
        // Для этого используем вспомогательный буфер, который vulkano сделает под капотом
        let mut upload_builder = AutoCommandBufferBuilder::primary(
            vulkan.cb_allocator.clone(),
            vulkan.queue.queue_family_index(),
            vulkano::command_buffer::CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        // Создаем staging буфер для копирования памяти на GPU
        let staging_buffer = Buffer::from_iter(
            vulkan.memory_allocator.clone(),
            BufferCreateInfo { usage: BufferUsage::TRANSFER_SRC, ..Default::default() },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            texture_data,
        ).unwrap();

        upload_builder.copy_buffer_to_image(vulkano::command_buffer::CopyBufferToImageInfo::buffer_image(
            staging_buffer,
            texture.clone(),
        )).unwrap();

        let upload_cb = upload_builder.build().unwrap();
        vulkano::sync::now(vulkan.device.clone())
            .then_execute(vulkan.queue.clone(), upload_cb).unwrap()
            .then_signal_fence_and_flush().unwrap()
            .wait(None).unwrap(); // Ждем завершения загрузки текстуры

        let texture_view = ImageView::new_default(texture.clone()).unwrap();

        // 3. Создаем Сэмплер (управляет фильтрацией)
        let sampler = Sampler::new(
            vulkan.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear, // Пиксели будут четкими (old-school)
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            }
        ).unwrap();

        // 4. Компиляция шейдеров и создание конвейера
        let vs = v_shader::load(vulkan.device.clone()).unwrap();
        let fs = f_shader::load(vulkan.device.clone()).unwrap();
        let vs_entry = vs.entry_point("main").unwrap();
        let fs_entry = fs.entry_point("main").unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs_entry.clone()),
            PipelineShaderStageCreateInfo::new(fs_entry),
        ];

        // Парсим шейдеры, извлекаем дескрипторы и генерируем PipelineLayoutCreateInfo
        let layout_create_info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(vulkan.device.clone())
            .unwrap();

        let layout = PipelineLayout::new(
            vulkan.device.clone(),
            layout_create_info
        ).unwrap();

        let pipeline = GraphicsPipeline::new(
            vulkan.device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(MyVertex::per_vertex().definition(&vs_entry).unwrap()),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(1, ColorBlendAttachmentState::default())),
                dynamic_state: [vulkano::pipeline::DynamicState::Viewport, vulkano::pipeline::DynamicState::Scissor].into_iter().collect(),
                subpass: Some(vulkano::pipeline::graphics::subpass::PipelineSubpassType::BeginRendering(
                    vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo {
                        color_attachment_formats: vec![Some(vulkan.image_format)],
                        ..Default::default()
                    }
                )),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        ).expect("failed to create graphics pipeline");

        // 5. Создаем Descriptor Set и связываем туда ImageView + Sampler
        let ds_allocator = Arc::new(StandardDescriptorSetAllocator::new(vulkan.device.clone(), Default::default()));
        // Специфика Vulkano: layout нужно склонировать как Arc, так как DescriptorSet забирает владение
        let ds_layout = pipeline.layout().set_layouts().get(0).unwrap().clone();

        // Описываем, что мы пишем в binding = 0
        let descriptor_writes = vec![
            WriteDescriptorSet::image_view_sampler(0, texture_view, sampler)
        ];

        // Нам нечего копировать из других сетов, поэтому массив пустой
        let descriptor_copies = vec![];

        let descriptor_set = DescriptorSet::new(
            ds_allocator,
            ds_layout,
            descriptor_writes,
            descriptor_copies,
        ).unwrap();

        self.vertex_buffer = Some(vertex_buffer);
        self.pipeline = Some(pipeline);
        self._texture = Some(texture);
        self.descriptor_set = Some(descriptor_set);
    }

    fn render(&mut self, _vulkan: &mut VulkanContext, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        // Просто пишем свои команды отрисовки в предоставленный движком билдер
        if let (Some(pipeline), Some(vertex_buffer), Some(descriptor_set)) = (&self.pipeline, &self.vertex_buffer, &self.descriptor_set) {
            builder
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap()
                // --- СВЯЗЫВАЕМ ТЕКСТУРУ С ШЕЙДЕРОМ ---
                .bind_descriptor_sets(
                    vulkano::pipeline::PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0, // Начинаем с set = 0
                    descriptor_set.clone(),
                )
                .unwrap();

            unsafe {
                builder.draw(6, 1, 0, 0).unwrap();
            }
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        println!("Game Resize: {}x{}", width, height);
    }

    fn shutdown(&mut self) {
        println!("Game Shutdown");
    }
}