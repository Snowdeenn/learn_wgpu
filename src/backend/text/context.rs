use new_collection::Arena;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::backend::context;

use super::resource::TextResource;

pub(super) struct TextContext {
    pub(super) font_system: glyphon::FontSystem,
    pub(super) swash_cache: glyphon::SwashCache,
    cache: glyphon::Cache,
    pub(super) viewport: glyphon::Viewport,
    pub(super) atlas: glyphon::TextAtlas,
    pub(super) renderer: glyphon::TextRenderer,
    pub(super) texts: Arena<TextResource>,
    pub(super) frame_state: super::draw::FrameState,
}

static CONTEXT: OnceLock<Mutex<TextContext>> = OnceLock::new();

pub(super) fn try_context() -> Result<MutexGuard<'static, TextContext>, super::TextError> {
    CONTEXT
        .get()
        .ok_or(super::TextError::NotInitialized)?
        .lock()
        .map_err(|_| super::TextError::Poisoned)
}

pub(super) fn reset_frame() {
    if let Some(context) = CONTEXT.get() {
        let mut context = context.lock().expect("Text context mutex is poisoned");
        if context.frame_state != super::draw::FrameState::Idle {
            context.atlas.trim();
            context.frame_state = super::draw::FrameState::Idle;
        }
    }
}

impl TextContext {
    fn init(format: wgpu::TextureFormat) -> Self {
        let ctx = context::context();
        let fs = glyphon::FontSystem::new();
        let sc = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&ctx.device);
        let vp = glyphon::Viewport::new(&ctx.device, &cache);
        let mut atlas = glyphon::TextAtlas::new(&ctx.device, &ctx.queue, &cache, format);
        let tr = glyphon::TextRenderer::new(
            &mut atlas,
            &ctx.device,
            wgpu::MultisampleState::default(),
            None,
        );
        let texts = Arena::new();

        TextContext {
            font_system: fs,
            swash_cache: sc,
            cache,
            viewport: vp,
            atlas,
            renderer: tr,
            texts,
            frame_state: super::draw::FrameState::Idle,
        }
    }

    pub(super) fn font_system(&self) -> &glyphon::FontSystem {
        &self.font_system
    }

    pub(super) fn font_system_mut(&mut self) -> &mut glyphon::FontSystem {
        &mut self.font_system
    }

    pub(super) fn swash_cache(&self) -> &glyphon::SwashCache {
        &self.swash_cache
    }

    pub(super) fn swash_cache_mut(&mut self) -> &mut glyphon::SwashCache {
        &mut self.swash_cache
    }

    pub(super) fn cache(&self) -> &glyphon::Cache {
        &self.cache
    }

    pub(super) fn cache_mut(&mut self) -> &mut glyphon::Cache {
        &mut self.cache
    }

    pub(super) fn viewport(&self) -> &glyphon::Viewport {
        &self.viewport
    }

    pub(super) fn viewport_mut(&mut self) -> &mut glyphon::Viewport {
        &mut self.viewport
    }

    pub(super) fn atlas(&self) -> &glyphon::TextAtlas {
        &self.atlas
    }

    pub(super) fn atlas_mut(&mut self) -> &mut glyphon::TextAtlas {
        &mut self.atlas
    }

    pub(super) fn renderer(&self) -> &glyphon::TextRenderer {
        &self.renderer
    }

    pub(super) fn renderer_mut(&mut self) -> &mut glyphon::TextRenderer {
        &mut self.renderer
    }

    pub(super) fn texts(&self) -> &Arena<TextResource> {
        &self.texts
    }

    pub(super) fn texts_mut(&mut self) -> &mut Arena<TextResource> {
        &mut self.texts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitError {
    AlreadyInitialized,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => f.write_str("Text context already initialized"),
        }
    }
}

impl std::error::Error for InitError {}

pub fn init(format: wgpu::TextureFormat) -> Result<(), InitError> {
    if CONTEXT.get().is_some() {
        return Err(InitError::AlreadyInitialized);
    }

    CONTEXT
        .set(Mutex::new(TextContext::init(format)))
        .map_err(|_| InitError::AlreadyInitialized)
}

/// Locks the text context. Do not call while already holding its guard.
pub(super) fn context() -> MutexGuard<'static, TextContext> {
    CONTEXT
        .get()
        .expect("Text context is not initialized")
        .lock()
        .expect("Text context mutex is poisoned")
}
