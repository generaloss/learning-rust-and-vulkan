use vulkano::buffer::{BufferContents};
use crate::buffer::BufferStorage;
use crate::index_buffer::{IndexBuffer, IndexType};
use crate::vertex_buffer::VertexBuffer;
use crate::vulkan_context::VulkanContext;

pub struct MeshIndexed<V, I> {
    pub vertex_buffer: VertexBuffer<V>,
    pub index_buffer: IndexBuffer<I>,
}

impl<V: BufferContents + 'static, I: IndexType> MeshIndexed<V, I> {
    pub fn from_data(vulkan: &VulkanContext, vertices: Vec<V>, indices: Vec<I>) -> Self {
        let vertex_buffer = VertexBuffer::from_data(
            vulkan.memory_allocator.clone(),
            vulkan.cb_allocator.clone(),
            vulkan.queue.clone(),
            vertices,
            BufferStorage::DeviceLocal
        ).unwrap();

        let index_buffer = IndexBuffer::from_data(
            vulkan.memory_allocator.clone(),
            vulkan.cb_allocator.clone(),
            vulkan.queue.clone(),
            indices,
            BufferStorage::DeviceLocal
        ).unwrap();

        Self {
            vertex_buffer,
            index_buffer,
        }
    }
}