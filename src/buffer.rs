//! buffer.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStorage {
    HostVisible, // Память доступна для записи с CPU.
    DeviceLocal, // Быстрая локальная видеопамять. Для загрузки используется staging-буфер.
}