use super::TextId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSpace {
    #[default]
    Screen,
    World,
}

/// Clip rectangle in logical screen pixels, regardless of the text's space.
#[derive(Clone, Copy, Debug)]
pub struct TextClip {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct TextPlacement {
    pub position: [f32; 2],
    pub scale: f32,
    /// Straight (unpremultiplied) sRGB RGBA, clamped to [0, 1].
    pub color: [f32; 4],
    pub space: TextSpace,
    /// Normalized anchor in the layout box: [0, 0] top-left, [0.5, 0.5] center.
    pub anchor: [f32; 2],
    pub clip: Option<TextClip>,
}

impl Default for TextPlacement {
    fn default() -> Self {
        Self {
            position: [0.0; 2],
            scale: 1.0,
            color: [1.0; 4],
            space: TextSpace::Screen,
            anchor: [0.0; 2],
            clip: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Text {
    pub text: TextId,
    pub position: [f32; 2],
    pub scale: f32,
    pub color: [f32; 4],
    pub space: TextSpace,
    pub anchor: [f32; 2],
    pub clip: Option<TextClip>,
}

impl Text {
    pub fn new(text: TextId, space: TextSpace) -> Self {
        let placement = TextPlacement::default();
        Self {
            text,
            position: placement.position,
            scale: placement.scale,
            color: placement.color,
            space,
            anchor: placement.anchor,
            clip: placement.clip,
        }
    }
}

pub trait TextInstance {
    fn text(&self) -> TextId;
    fn placement(&self) -> TextPlacement;
}

impl TextInstance for Text {
    fn text(&self) -> TextId {
        self.text
    }

    fn placement(&self) -> TextPlacement {
        TextPlacement {
            position: self.position,
            scale: self.scale,
            color: self.color,
            space: self.space,
            anchor: self.anchor,
            clip: self.clip,
        }
    }
}
