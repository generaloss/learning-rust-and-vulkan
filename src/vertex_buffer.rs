// 'vertex_buffer.rs'

use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use crate::vulkan_context::VulkanContext;

pub struct VertexBuffer<V> {
    pub subbuffer: Subbuffer<[V]>,
}

impl<V: BufferContents + 'static> VertexBuffer<V> {
    pub fn new<I>(vulkan_context: &VulkanContext, data: I) -> Self where I: IntoIterator<Item = V>, I::IntoIter: ExactSizeIterator {
        let subbuffer = Buffer::from_iter(
            vulkan_context.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        ).unwrap();

        Self {
            subbuffer
        }
    }
}