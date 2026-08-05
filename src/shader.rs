// 'engine/shader.rs'

use std::sync::Arc;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo, Pipeline};
use vulkano::pipeline::graphics::vertex_input::VertexInputState;
use vulkano::shader::EntryPoint;
use crate::vulkan_context::VulkanContext;

pub struct Shader {
    pub pipeline: Arc<GraphicsPipeline>,
}

impl Shader {
    pub fn new(
        vulkan_context: &VulkanContext,
        vs_entry: EntryPoint,
        fs_entry: EntryPoint,
        vertex_definition: VertexInputState,
    ) -> Self {

        let stages = [
            PipelineShaderStageCreateInfo::new(vs_entry.clone()),
            PipelineShaderStageCreateInfo::new(fs_entry),
        ];

        // Автоматически строим макет на основе анализа кода шейдеров
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
                vertex_input_state: Some(vertex_definition),
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

        Self { pipeline }
    }

    pub fn layout(&self) -> &Arc<PipelineLayout> {
        self.pipeline.layout()
    }
}