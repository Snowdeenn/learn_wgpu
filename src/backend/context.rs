use new_collection::{Arena, Id};
use std::sync::{Arc, OnceLock, RwLock};

use crate::backend::buffer::BufferArena;

use super::texture::TextureResource;

type TextureStorage = Arena<TextureResource>;

#[derive(Debug)]
pub struct Context {
    pub queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub texture_storage: Arc<RwLock<TextureStorage>>,
    pub(super) buffer: Arc<RwLock<BufferArena>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContextId {
    raw: Id<Arc<Context>>,
    uid: u64,
}

pub fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

static DEFAULT_CONTEXT: OnceLock<Arc<Context>> = OnceLock::new();
static CONTEXTS: OnceLock<RwLock<Arena<Arc<Context>>>> = OnceLock::new();

impl Context {
    fn init(adapter: &wgpu::Adapter) -> Self {
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Main device"),
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .expect("Impossible de creer le device");

        let texture_storage = Arc::new(RwLock::new(TextureStorage::new()));
        let buffer = Arc::new(RwLock::new(BufferArena::new()));

        Context {
            queue,
            device,
            texture_storage,
            buffer,
        }
    }
}

/// Initializes the backend's default context.
pub fn init(adapter: &wgpu::Adapter) {
    DEFAULT_CONTEXT
        .set(Arc::new(Context::init(adapter)))
        .expect("Default context already init");
}

fn contexts() -> &'static RwLock<Arena<Arc<Context>>> {
    CONTEXTS.get_or_init(|| RwLock::new(Arena::new()))
}

pub fn create(adapter: &wgpu::Adapter) -> ContextId {
    let new_context = Context::init(adapter);
    let raw = {
        let mut ctxs = contexts().write().unwrap();
        ctxs.insert(Arc::new(new_context))
    };

    ContextId {
        raw,
        uid: next_id(),
    }
}

pub fn context(id: Option<ContextId>) -> Arc<Context> {
    match id {
        None => Arc::clone(DEFAULT_CONTEXT.get().expect("Default context is not init")),
        Some(id) => {
            let ctxs = contexts().read().unwrap();
            Arc::clone(ctxs.get(id.raw).expect("Invalid ContextId: context missing or removed"))
        }
    }
}

pub fn device(id: Option<ContextId>) -> wgpu::Device {
    context(id).device.clone()
}

pub fn queue(id: Option<ContextId>) -> wgpu::Queue {
    context(id).queue.clone()
}

pub fn texture_storage(id: Option<ContextId>) -> Arc<RwLock<TextureStorage>> {
    Arc::clone(&context(id).texture_storage)
}

pub(super) fn buffer(id: Option<ContextId>) -> Arc<RwLock<BufferArena>> {
    Arc::clone(&context(id).buffer)
}