use super::{TextError, TextId, TextInstance, TextPlacement, TextSpace, context};
use crate::{backend::context as gpu, camera::Camera2D};

/// Physical target size, window DPI and the same camera used for the scene.
pub struct TextView<'a> {
    pub resolution: [u32; 2],
    pub scale_factor: f32,
    pub camera: &'a Camera2D,
}

impl TextView<'_> {
    pub(super) fn validate(&self) -> Result<(), TextError> {
        if self
            .resolution
            .iter()
            .any(|&size| size == 0 || size > i32::MAX as u32)
            || !self.scale_factor.is_finite()
            || self.scale_factor <= 0.0
            || !self.camera.pos().x.is_finite()
            || !self.camera.pos().y.is_finite()
        {
            return Err(TextError::InvalidView);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrameState {
    Idle,
    Failed,
    Prepared,
    Drawn,
}

impl FrameState {
    pub(super) fn begin(&mut self) -> Result<(), TextError> {
        if *self != Self::Idle {
            return Err(TextError::FramePending);
        }
        // Any validation or Glyphon failure leaves this frame undrawable.
        *self = Self::Failed;
        Ok(())
    }

    pub(super) fn can_draw(&self) -> Result<(), TextError> {
        match self {
            Self::Prepared => Ok(()),
            Self::Drawn => Err(TextError::AlreadyDrawn),
            _ => Err(TextError::NotPrepared),
        }
    }
}

pub(super) struct ResolvedPlacement {
    pub position: [f32; 2],
    pub scale: f32,
    pub bounds: glyphon::TextBounds,
    pub color: glyphon::Color,
}

pub(super) fn resolve(
    placement: TextPlacement,
    size: [f32; 2],
    view: &TextView<'_>,
) -> Result<ResolvedPlacement, TextError> {
    if !placement
        .position
        .iter()
        .chain(placement.color.iter())
        .all(|v| v.is_finite())
        || !placement
            .anchor
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        || !placement.scale.is_finite()
        || placement.scale <= 0.0
    {
        return Err(TextError::InvalidPlacement);
    }
    let (position, scale) = match placement.space {
        TextSpace::Screen => (
            placement.position.map(|v| v * view.scale_factor),
            placement.scale * view.scale_factor,
        ),
        TextSpace::World => (
            view.camera
                .world_to_screen(placement.position, view.resolution),
            placement.scale * view.camera.zoom(),
        ),
    };
    let position = [
        position[0] - size[0] * placement.anchor[0] * scale,
        position[1] - size[1] * placement.anchor[1] * scale,
    ];
    if !scale.is_finite() || !position.iter().all(|v| v.is_finite()) {
        return Err(TextError::InvalidPlacement);
    }
    let mut bounds = glyphon::TextBounds {
        left: 0,
        top: 0,
        right: view.resolution[0] as i32,
        bottom: view.resolution[1] as i32,
    };
    if let Some(clip) = placement.clip {
        if !clip
            .min
            .iter()
            .chain(clip.max.iter())
            .all(|v| v.is_finite())
            || clip.min[0] > clip.max[0]
            || clip.min[1] > clip.max[1]
        {
            return Err(TextError::InvalidPlacement);
        }
        bounds.left = ((clip.min[0] * view.scale_factor).floor() as i32).clamp(0, bounds.right);
        bounds.top = ((clip.min[1] * view.scale_factor).floor() as i32).clamp(0, bounds.bottom);
        bounds.right =
            ((clip.max[0] * view.scale_factor).ceil() as i32).clamp(bounds.left, bounds.right);
        bounds.bottom =
            ((clip.max[1] * view.scale_factor).ceil() as i32).clamp(bounds.top, bounds.bottom);
    }
    let [r, g, b, a] = placement
        .color
        .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    Ok(ResolvedPlacement {
        position,
        scale,
        bounds,
        color: glyphon::Color::rgba(r, g, b, a),
    })
}

pub(super) fn ordered_instances<T: TextInstance>(instances: &[T]) -> Vec<(TextId, TextPlacement)> {
    let mut ordered: Vec<_> = instances
        .iter()
        .map(|text| (text.text(), text.placement()))
        .collect();
    // Stable sort: preserve overlap order within each space, including repeated IDs.
    ordered.sort_by_key(|(_, placement)| placement.space == TextSpace::Screen);
    ordered
}

/// Prepare exactly once per frame, including for an empty list.
/// A failed attempt must be followed by submission or discard before retrying.
pub fn prepare<T: TextInstance>(instances: &[T], view: &TextView<'_>) -> Result<(), TextError> {
    let mut context = context::try_context()?;
    context.frame_state.begin()?;
    view.validate()?;
    let context::TextContext {
        font_system,
        swash_cache,
        viewport,
        atlas,
        renderer,
        texts,
        frame_state,
        ..
    } = &mut *context;
    let instances = ordered_instances(instances);
    for (id, _) in &instances {
        // Cosmic-text resolves only dirty buffers; moving instances does not reshape text.
        texts
            .get_mut(*id)
            .ok_or(TextError::InvalidId(*id))?
            .buffer
            .shape_until_scroll(font_system, false);
    }
    let mut areas = Vec::with_capacity(instances.len());
    for (id, placement) in instances {
        let text = texts.get(id).ok_or(TextError::InvalidId(id))?;
        let placement = resolve(placement, text.size(), view)?;
        areas.push(glyphon::TextArea {
            buffer: &text.buffer,
            left: placement.position[0],
            top: placement.position[1],
            scale: placement.scale,
            bounds: placement.bounds,
            default_color: placement.color,
            custom_glyphs: &[],
        });
    }
    viewport.update(
        gpu::queue(),
        glyphon::Resolution {
            width: view.resolution[0],
            height: view.resolution[1],
        },
    );
    renderer
        .prepare(
            gpu::device(),
            gpu::queue(),
            font_system,
            atlas,
            viewport,
            areas,
            swash_cache,
        )
        .map_err(TextError::Prepare)?;
    *frame_state = FrameState::Prepared;
    Ok(())
}

pub fn draw_all(pass: &mut wgpu::RenderPass<'_>) -> Result<(), TextError> {
    let mut context = context::try_context()?;
    context.frame_state.can_draw()?;
    context.frame_state = FrameState::Failed;
    context
        .renderer
        .render(&context.atlas, &context.viewport, pass)
        .map_err(TextError::Render)?;
    context.frame_state = FrameState::Drawn;
    Ok(())
}

/// Call only after submitting the command buffer containing the text draw.
pub fn submitted() {
    context::reset_frame();
}

/// Call only after dropping all encoders/command buffers for the abandoned frame.
/// Those commands must never be submitted after another text batch is prepared.
pub fn discard() {
    context::reset_frame();
}

/// Declared after Frame's encoder so an abandoned encoder is dropped first.
pub(crate) struct FrameCleanup;

impl Drop for FrameCleanup {
    fn drop(&mut self) {
        discard();
    }
}
