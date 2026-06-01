// 'main.rs'

pub mod engine;

use std::sync::Arc;
use winit::error::EventLoopError;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::layout::PipelineLayoutCreateInfo;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::buffer::BufferContents;
use vulkano::pipeline::graphics::vertex_input::{Vertex as VulkanoVertex, VertexDefinition};

use engine::context_builder::ContextBuilder;
use engine::app_adapter::AppAdapter;
use crate::engine::context_manager::ContextManager;
use crate::engine::vulkan_context::VulkanContext;

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r#"
            #version 450
            layout(location = 0) in vec2 position;
            layout(location = 1) in vec3 color;
            layout(location = 0) out vec3 v_color;
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                v_color = color;
            }
        "#,
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r#"
            #version 450
            layout(location = 0) in vec3 v_color;
            layout(location = 0) out vec4 f_color;
            void main() {
                f_color = vec4(v_color, 1.0);
            }
        "#,
    }
}

#[derive(BufferContents, VulkanoVertex, Clone, Copy, Debug, Default)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    color: [f32; 3],
}

fn main() -> Result<(), EventLoopError> {
    let mut context = ContextBuilder::new()
        .title("Blazing Fast Engine")
        .size(1280, 720)
        .create();

    context.set_app(Game::new());

    let mut manager = ContextManager::new();
    manager.register(context);
    manager.run()?;
    Ok(())
}

struct Game {
    pipeline: Option<Arc<GraphicsPipeline>>,
    vertex_buffer: Option<Subbuffer<[MyVertex]>>,
}

impl Game {
    fn new() -> Self {
        Self { pipeline: None, vertex_buffer: None }
    }
}

impl AppAdapter for Game {
    fn init(&mut self, vulkan: &mut VulkanContext) {
        println!("Game Init: Creating pipeline and resources...");

        // Создаем буфер вершин, используя аллокатор контекста
        let vertices = [
            MyVertex { position: [ 0.0, -0.5], color: [1.0, 0.0, 0.0] },
            MyVertex { position: [ 0.5,  0.5], color: [0.0, 1.0, 0.0] },
            MyVertex { position: [-0.5,  0.5], color: [0.0, 0.0, 1.0] },
        ];

        let vertex_buffer = Buffer::from_iter(
            vulkan.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices,
        ).unwrap();

        // Загружаем шейдеры, используя устройство контекста
        let vs = vs::load(vulkan.device.clone()).unwrap();
        let fs = fs::load(vulkan.device.clone()).unwrap();

        let vs_entry = vs.entry_point("main").unwrap();
        let fs_entry = fs.entry_point("main").unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs_entry.clone()),
            PipelineShaderStageCreateInfo::new(fs_entry),
        ];

        let layout = PipelineLayout::new(
            vulkan.device.clone(),
            PipelineLayoutCreateInfo::default()
        ).unwrap();

        let pipeline = GraphicsPipeline::new(
            vulkan.device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(MyVertex::per_vertex().definition(&vs_entry).unwrap()),
                input_assembly_state: Some(InputAssemblyState::default().topology(PrimitiveTopology::TriangleList)),
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

        self.vertex_buffer = Some(vertex_buffer);
        self.pipeline = Some(pipeline);
    }

    fn render(&mut self, _vulkan: &mut VulkanContext, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        // Просто пишем свои команды отрисовки в предоставленный движком билдер
        if let (Some(pipeline), Some(vertex_buffer)) = (&self.pipeline, &self.vertex_buffer) {
            builder
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap();

            unsafe {
                builder.draw(3, 1, 0, 0).unwrap();
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