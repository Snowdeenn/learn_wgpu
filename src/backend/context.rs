use new_collection::Arena;
use std::sync::{OnceLock, RwLock};

use crate::backend::buffer::BufferArena;

use super::texture::TextureResource;

type TextureStorage = Arena<TextureResource>;

#[derive(Debug)]
pub struct Context {
    pub queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub texture_storage: RwLock<TextureStorage>,
    pub(super) buffer: RwLock<BufferArena>,
}

static CONTEXT: OnceLock<Context> = OnceLock::new();

impl Context {
    fn init(adapter: &wgpu::Adapter) -> Self {
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Main device"),
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .expect("Impossible de creer le device");

        let texture_storage = RwLock::new(TextureStorage::new());
        let buffer = RwLock::new(BufferArena::new());

        Context {
            queue,
            device,
            texture_storage,
            buffer,
        }
    }
}

pub fn init(adapter: &wgpu::Adapter) {
    CONTEXT
        .set(Context::init(adapter))
        .expect("Context already init");
}

pub fn context() -> &'static Context {
    CONTEXT.get().expect("Context is not init")
}

pub fn device() -> &'static wgpu::Device {
    &CONTEXT.get().expect("Context is not init").device
}

pub fn queue() -> &'static wgpu::Queue {
    &CONTEXT.get().expect("Context is not init").queue
}

pub fn texture_storage() -> &'static RwLock<TextureStorage> {
    &CONTEXT.get().expect("Context is not init").texture_storage
}

pub(super) fn buffer() -> &'static RwLock<BufferArena> {
    &CONTEXT.get().expect("Context is not init").buffer
}
