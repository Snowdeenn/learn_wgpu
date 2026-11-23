use new_collection::{Arena, Id};
use std::sync::{Arc, OnceLock, RwLock};

use crate::backend::buffer::BufferArena;
use crate::{OwnedResource, ResourceHandle};

#[derive(Debug)]
pub struct Context {
    pub queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub(super) buffer: Arc<RwLock<BufferArena>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextId {
    raw: Id<Arc<Context>>,
    uid: u64,
}

impl ResourceHandle for ContextId {
    fn release(self) {
        remove(self);
    }
}

pub fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

static DEFAULT_CONTEXT: OnceLock<Arc<Context>> = OnceLock::new();
static CONTEXTS: OnceLock<RwLock<Arena<Arc<Context>>>> = OnceLock::new();

impl Context {
    fn init(adapter: &wgpu::Adapter) -> Result<Self, InitError> {
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Main device"),
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(InitError::Device)?;

        let buffer = Arc::new(RwLock::new(BufferArena::new()));

        Ok(Context {
            queue,
            device,
            buffer,
        })
    }
}

/// Initializes the backend's default context.
pub fn init(adapter: &wgpu::Adapter) -> Result<(), InitError> {
    if DEFAULT_CONTEXT.get().is_some() {
        return Err(InitError::AlreadyInitialized);
    }
    DEFAULT_CONTEXT
        .set(Arc::new(Context::init(adapter)?))
        .map_err(|_| InitError::AlreadyInitialized)
}

/// Adopt an application GPU, preserving its existing surface and event loop.
pub fn init_from(device: wgpu::Device, queue: wgpu::Queue) -> Result<(), InitError> {
    DEFAULT_CONTEXT
        .set(Arc::new(Context {
            device,
            queue,
            buffer: Arc::new(RwLock::new(BufferArena::new())),
        }))
        .map_err(|_| InitError::AlreadyInitialized)
}

fn contexts() -> &'static RwLock<Arena<Arc<Context>>> {
    CONTEXTS.get_or_init(|| RwLock::new(Arena::new()))
}

pub fn create(adapter: &wgpu::Adapter) -> Result<OwnedResource<ContextId>, InitError> {
    let new_context = Context::init(adapter)?;
    let raw = {
        let mut ctxs = contexts().write().unwrap();
        ctxs.insert(Arc::new(new_context))
    };

    Ok(OwnedResource::new(ContextId {
        raw,
        uid: next_id(),
    }))
}

/// Registers an application-owned device/queue pair (for incremental integrations).
/// Both handles must come from the same device.
pub fn register(device: wgpu::Device, queue: wgpu::Queue) -> OwnedResource<ContextId> {
    let raw = contexts().write().unwrap().insert(Arc::new(Context {
        queue,
        device,
        buffer: Arc::new(RwLock::new(BufferArena::new())),
    }));
    OwnedResource::new(ContextId {
        raw,
        uid: next_id(),
    })
}

/// Fallible counterpart used by the described rendering API.
pub fn try_context(id: Option<ContextId>) -> Option<Arc<Context>> {
    match id {
        None => DEFAULT_CONTEXT.get().cloned(),
        Some(id) => contexts().read().unwrap().get(id.raw).cloned(),
    }
}

pub fn remove(id: ContextId) -> Option<Arc<Context>> {
    let mut contexts = contexts().write().unwrap();
    contexts.get(id.raw)?;
    contexts.remove(id.raw)
}

#[derive(Debug)]
pub enum InitError {
    Device(wgpu::RequestDeviceError),
    AlreadyInitialized,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(error) => write!(f, "Device creation failed: {error}"),
            Self::AlreadyInitialized => f.write_str("Default context already initialized"),
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Device(error) => Some(error),
            _ => None,
        }
    }
}

pub fn context(id: Option<ContextId>) -> Arc<Context> {
    match id {
        None => Arc::clone(DEFAULT_CONTEXT.get().expect("Default context is not init")),
        Some(id) => {
            let ctxs = contexts().read().unwrap();
            Arc::clone(
                ctxs.get(id.raw)
                    .expect("Invalid ContextId: context missing or removed"),
            )
        }
    }
}

pub fn device(id: Option<ContextId>) -> wgpu::Device {
    context(id).device.clone()
}

pub fn queue(id: Option<ContextId>) -> wgpu::Queue {
    context(id).queue.clone()
}

pub(super) fn buffer(id: Option<ContextId>) -> Arc<RwLock<BufferArena>> {
    Arc::clone(&context(id).buffer)
}
