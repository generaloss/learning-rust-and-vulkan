//! index_buffer.rs

use std::error::Error;
use std::ops::Deref;
use std::sync::Arc;

use vulkano::buffer::{
    Buffer,
    BufferContents,
    BufferCreateInfo,
    BufferUsage,
    IndexBuffer as VulkanoIndexBuffer,
    Subbuffer,
};
use vulkano::command_buffer::{
    allocator::StandardCommandBufferAllocator,
    AutoCommandBufferBuilder,
    CommandBufferUsage,
    CopyBufferInfo,
    PrimaryCommandBufferAbstract,
};
use vulkano::device::Queue;
use vulkano::memory::allocator::{
    AllocationCreateInfo,
    MemoryTypeFilter,
    StandardMemoryAllocator,
};
use vulkano::sync::GpuFuture;
use crate::buffer::BufferStorage;

/// Обертка над Vulkano IndexBuffer.
#[derive(Debug, Clone)]
pub struct IndexBuffer<I> {
    buffer: Subbuffer<[I]>,
    storage: BufferStorage,
}

impl<I> Deref for IndexBuffer<I> {
    type Target = Subbuffer<[I]>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

/// Типы индексов, которые поддерживает Vulkan/Vulkano.
pub trait IndexType: BufferContents + 'static {}

impl IndexType for u8 {}
impl IndexType for u16 {}
impl IndexType for u32 {}

impl<I: IndexType> IndexBuffer<I> {
    /// Создает индексный буфер из итератора.
    pub fn from_data<T>(
        allocator: Arc<StandardMemoryAllocator>,
        cb_allocator: Arc<StandardCommandBufferAllocator>,
        queue: Arc<Queue>,
        data: T,
        storage: BufferStorage,
    ) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        T: IntoIterator<Item = I>,
        T::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();

        let buffer = match storage {
            BufferStorage::HostVisible => Buffer::from_iter(
                allocator,
                BufferCreateInfo {
                    usage: BufferUsage::INDEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_HOST
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                data,
            )?,

            BufferStorage::DeviceLocal => {
                // 1. Создаем staging-буфер в host-visible памяти.
                let staging = Buffer::from_iter(
                    allocator.clone(),
                    BufferCreateInfo {
                        usage: BufferUsage::TRANSFER_SRC,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_HOST
                            | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    data,
                )?;

                // 2. Создаем настоящий индексный буфер в device-local памяти.
                let dst = Buffer::new_slice::<I>(
                    allocator,
                    BufferCreateInfo {
                        usage: BufferUsage::INDEX_BUFFER
                            | BufferUsage::TRANSFER_DST,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                        ..Default::default()
                    },
                    staging.len(),
                )?;

                // 3. Копируем staging -> device-local.
                upload_staging_sync(
                    cb_allocator,
                    queue,
                    staging,
                    dst.clone(),
                )?;

                dst
            }
        };

        Ok(Self { buffer, storage })
    }

    /// Создает Host-Visible индексный буфер.
    #[inline]
    pub fn host_visible<T>(
        allocator: Arc<StandardMemoryAllocator>,
        data: T,
    ) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        T: IntoIterator<Item = I>,
        T::IntoIter: ExactSizeIterator,
    {
        let buffer = Buffer::from_iter(
            allocator,
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        )?;

        Ok(Self {
            buffer,
            storage: BufferStorage::HostVisible,
        })
    }

    #[inline]
    pub fn storage(&self) -> BufferStorage {
        self.storage
    }

    #[inline]
    pub fn len(&self) -> u32 {
        self.buffer.len() as u32
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.len() == 0
    }

    /// Преобразует наш буфер в Vulkano IndexBuffer.
    ///
    /// Это именно тот тип, который нужно передавать
    /// в `AutoCommandBufferBuilder::bind_index_buffer`.
    #[inline]
    pub fn as_vulkano(&self) -> VulkanoIndexBuffer where vulkano::buffer::IndexBuffer: From<Subbuffer<[I]>> {
        self.buffer.clone().into()
    }

    #[inline]
    pub fn into_subbuffer(self) -> Subbuffer<[I]> {
        self.buffer
    }

    #[inline]
    pub fn into_vulkano(self) -> VulkanoIndexBuffer where vulkano::buffer::IndexBuffer: From<Subbuffer<[I]>>  {
        self.buffer.into()
    }
}

/// Выполняет синхронный трансфер staging -> device-local.
fn upload_staging_sync<I: BufferContents + 'static>(
    cb_allocator: Arc<StandardCommandBufferAllocator>,
    queue: Arc<Queue>,
    src: Subbuffer<[I]>,
    dst: Subbuffer<[I]>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut builder = AutoCommandBufferBuilder::primary(
        cb_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )?;

    builder.copy_buffer(CopyBufferInfo::buffers(src, dst))?;

    let command_buffer = builder.build()?;

    command_buffer
        .execute(queue)?
        .then_signal_fence_and_flush()?
        .wait(None)?;

    Ok(())
}