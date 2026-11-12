mod context;
mod draw;
mod instance;
mod resource;

pub use context::{InitError, init};
pub(crate) use draw::FrameCleanup;
pub use draw::{TextView, discard, draw_all, prepare, submitted};
pub use glyphon::cosmic_text::Align;
pub use glyphon::{Family, Style, Weight, Wrap};
pub use instance::{Text, TextClip, TextInstance, TextPlacement, TextSpace};
pub use resource::{
    TextDescription, TextId, TextLayout, TextResource, register, remove, set_layout, set_text,
};

#[derive(Debug)]
pub enum TextError {
    NotInitialized,
    Poisoned,
    InvalidId(TextId),
    InvalidLayout,
    InvalidView,
    InvalidPlacement,
    FramePending,
    NotPrepared,
    AlreadyDrawn,
    Prepare(glyphon::PrepareError),
    Render(glyphon::RenderError),
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => f.write_str("Text context is not initialized"),
            Self::Poisoned => f.write_str("Text context mutex is poisoned"),
            Self::InvalidId(id) => write!(f, "Unknown text: {id}"),
            Self::InvalidLayout => f.write_str(
                "Text metrics must be finite and positive; width must be finite and nonnegative",
            ),
            Self::InvalidView => f.write_str("Invalid text viewport, DPI factor or camera"),
            Self::InvalidPlacement => {
                f.write_str("Invalid text position, scale, anchor, color or clip rectangle")
            }
            Self::FramePending => f.write_str(
                "Submit or discard the pending text frame before preparing another batch",
            ),
            Self::NotPrepared => f.write_str("No successfully prepared text batch"),
            Self::AlreadyDrawn => f.write_str("Text batch has already been drawn"),
            Self::Prepare(error) => write!(f, "Text preparation failed: {error}"),
            Self::Render(error) => write!(f, "Text rendering failed: {error}"),
        }
    }
}

impl std::error::Error for TextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Prepare(error) => Some(error),
            Self::Render(error) => Some(error),
            _ => None,
        }
    }
}
