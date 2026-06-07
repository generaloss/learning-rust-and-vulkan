// 'main.rs'

pub mod engine;

use crate::engine::vulkan_context::VulkanContext;

use std::sync::Arc;
use winit::error::EventLoopError;
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
use vulkano::pipeline::Pipeline;
use vulkano::descriptor_set::{WriteDescriptorSet, allocator::StandardDescriptorSetAllocator, DescriptorSet};
use winit::dpi::Pixel;
use crate::engine::application_context::{ContextBuilder, ContextManager, AppAdapter};
use crate::engine::camera::CameraOrthographic;
use crate::engine::texture::Texture;
use crate::engine::vertex_buffer::VertexBuffer;

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
    vertex_buffer: Option<VertexBuffer<MyVertex>>,
    texture: Option<Texture>,
    descriptor_set: Option<Arc<DescriptorSet>>,
    camera: CameraOrthographic,
}

impl BlazingFastApp {
    fn new() -> Self {
        Self {
            pipeline: None,
            vertex_buffer: None,
            texture: None,
            descriptor_set: None,
            camera: CameraOrthographic::new(),
        }
    }
}

impl AppAdapter for BlazingFastApp {
    fn init(&mut self, vulkan_context: &mut VulkanContext) {
        println!("Init");

        let vertex_buffer = VertexBuffer::new(vulkan_context, [
            MyVertex { a_pos: [0.0,   0.0  ], a_uv: [0.0, 1.0] },
            MyVertex { a_pos: [0.0,   400.0], a_uv: [0.0, 0.0] },
            MyVertex { a_pos: [400.0, 400.0], a_uv: [1.0, 0.0] },

            MyVertex { a_pos: [400.0, 400.0], a_uv: [1.0, 0.0] },
            MyVertex { a_pos: [400.0, 0.0  ], a_uv: [1.0, 1.0] },
            MyVertex { a_pos: [0.0,   0.0  ], a_uv: [0.0, 1.0] },
        ]);

        let texture = Texture::from_path(vulkan_context, "assets/image.jpg");

        // Compile shaders
        let vs = v_shader::load(vulkan_context.device.clone()).unwrap();
        let fs = f_shader::load(vulkan_context.device.clone()).unwrap();
        let vs_entry = vs.entry_point("main").unwrap();
        let fs_entry = fs.entry_point("main").unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs_entry.clone()),
            PipelineShaderStageCreateInfo::new(fs_entry),
        ];

        // Парсим шейдеры, извлекаем дескрипторы и генерируем PipelineLayoutCreateInfo
        let layout_create_info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(vulkan_context.device.clone())
            .unwrap();

        let layout = PipelineLayout::new(
            vulkan_context.device.clone(),
            layout_create_info
        ).unwrap();

        let pipeline = GraphicsPipeline::new(
            vulkan_context.device.clone(),
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
                        color_attachment_formats: vec![Some(vulkan_context.image_format)],
                        ..Default::default()
                    }
                )),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        ).expect("failed to create graphics pipeline");

        // 5. Создаем Descriptor Set и связываем туда ImageView + Sampler
        let ds_allocator = Arc::new(StandardDescriptorSetAllocator::new(vulkan_context.device.clone(), Default::default()));
        // Специфика Vulkano: layout нужно склонировать как Arc, так как DescriptorSet забирает владение
        let ds_layout = pipeline.layout().set_layouts().get(0).unwrap().clone();

        // Описываем, что мы пишем в binding = 0
        let descriptor_writes = vec![
            WriteDescriptorSet::image_view_sampler(0, texture.image_view.clone(), texture.sampler.clone())
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
        self.texture = Some(texture);
        self.descriptor_set = Some(descriptor_set);
    }

    fn render(&mut self, _vulkan: &mut VulkanContext, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if let (Some(pipeline), Some(vertex_buffer), Some(descriptor_set)) = (&self.pipeline, &self.vertex_buffer, &self.descriptor_set) {
            let push_constants = v_shader::PushConstants {
                u_combined: self.camera.combined.to_cols_array_2d(),
            };

            builder
                .bind_pipeline_graphics(pipeline.clone()).unwrap()
                .bind_vertex_buffers(0, vertex_buffer.subbuffer.clone()).unwrap()
                .push_constants(pipeline.layout().clone(), 0, push_constants).unwrap()
                .bind_descriptor_sets(
                    vulkano::pipeline::PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0, // Начинаем с set = 0
                    descriptor_set.clone(),
                ).unwrap();

            unsafe {
                builder.draw(6, 1, 0, 0).unwrap();
            }
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.camera.resize(width as f32, height as f32);
    }

    fn shutdown(&mut self) {
        println!("Shutdown");
    }
}