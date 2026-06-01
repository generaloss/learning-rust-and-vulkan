// 'app_adapter.rs'

use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use crate::engine::vulkan_context::VulkanContext;

pub trait AppAdapter {

    fn init(&mut self, vulkan: &mut VulkanContext);
    fn render(&mut self, vulkan: &mut VulkanContext, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>);
    fn resize(&mut self, width: u32, height: u32);
    fn shutdown(&mut self);

}