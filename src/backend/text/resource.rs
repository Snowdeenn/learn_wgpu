use glyphon::{Attrs, AttrsOwned, Buffer, FontSystem, Metrics, Shaping};
use new_collection::Id;

use super::{Align, Family, Style, TextError, Weight, Wrap, context};

pub struct TextResource {
    pub(super) buffer: Buffer,
}

pub type TextId = Id<TextResource>;

/// Metrics are logical pixels for screen instances and world units for world instances.
#[derive(Clone, Copy, Debug)]
pub struct TextLayout {
    pub font_size: f32,
    pub line_height: f32,
    pub max_width: Option<f32>,
    /// None follows paragraph direction (LTR or RTL).
    pub alignment: Option<Align>,
    pub wrap: Wrap,
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            font_size: 24.0,
            line_height: 30.0,
            max_width: None,
            alignment: None,
            wrap: Wrap::WordOrGlyph,
        }
    }
}

impl TextLayout {
    pub(super) fn validate(&self) -> Result<(), TextError> {
        if !self.font_size.is_finite()
            || self.font_size <= 0.0
            || !self.line_height.is_finite()
            || self.line_height <= 0.0
            || self
                .max_width
                .is_some_and(|width| !width.is_finite() || width < 0.0)
        {
            return Err(TextError::InvalidLayout);
        }
        Ok(())
    }
}

/// Fonts are discovered from the operating system; unavailable families use font fallback.
#[derive(Clone, Copy, Debug)]
pub struct TextDescription<'a> {
    pub content: &'a str,
    pub font: Family<'a>,
    pub weight: Weight,
    pub style: Style,
    pub font_size: f32,
    pub line_height: f32,
    pub max_width: Option<f32>,
    pub alignment: Option<Align>,
    pub wrap: Wrap,
}

impl Default for TextDescription<'_> {
    fn default() -> Self {
        let layout = TextLayout::default();
        Self {
            content: "",
            font: Family::SansSerif,
            weight: Weight::NORMAL,
            style: Style::Normal,
            font_size: layout.font_size,
            line_height: layout.line_height,
            max_width: layout.max_width,
            alignment: layout.alignment,
            wrap: layout.wrap,
        }
    }
}

impl TextResource {
    pub(super) fn new(
        font_system: &mut FontSystem,
        description: TextDescription<'_>,
    ) -> Result<Self, TextError> {
        let layout = TextLayout {
            font_size: description.font_size,
            line_height: description.line_height,
            max_width: description.max_width,
            alignment: description.alignment,
            wrap: description.wrap,
        };
        layout.validate()?;
        let mut buffer = Buffer::new(
            font_system,
            Metrics::new(layout.font_size, layout.line_height),
        );
        buffer.set_size(layout.max_width, None);
        buffer.set_wrap(layout.wrap);
        buffer.set_text(
            description.content,
            &Attrs::new()
                .family(description.font)
                .weight(description.weight)
                .style(description.style),
            Shaping::Advanced,
            layout.alignment,
        );
        Ok(Self { buffer })
    }

    pub(super) fn set_text(&mut self, content: &str) {
        // Keep style and alignment in the Buffer, even when replacing empty text.
        let line = self
            .buffer
            .lines
            .first()
            .expect("Text buffers contain at least one line");
        let attrs = AttrsOwned::new(&line.attrs_list().defaults());
        let alignment = line.align();
        self.buffer
            .set_text(content, &attrs.as_attrs(), Shaping::Advanced, alignment);
    }

    pub(super) fn set_layout(&mut self, layout: TextLayout) -> Result<(), TextError> {
        layout.validate()?;
        self.buffer.set_metrics_and_size(
            Metrics::new(layout.font_size, layout.line_height),
            layout.max_width,
            None,
        );
        self.buffer.set_wrap(layout.wrap);
        for line in &mut self.buffer.lines {
            line.set_align(layout.alignment);
        }
        Ok(())
    }

    /// Layout box, including the configured width and all line heights.
    pub(super) fn size(&self) -> [f32; 2] {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for run in self.buffer.layout_runs() {
            width = width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
        }
        [self.buffer.size().0.unwrap_or(width), height]
    }
}

pub fn register(description: TextDescription<'_>) -> Result<TextId, TextError> {
    let mut context = context::try_context()?;
    let text = TextResource::new(&mut context.font_system, description)?;
    Ok(context.texts.insert(text))
}

pub fn set_text(id: TextId, content: &str) -> Result<(), TextError> {
    let mut context = context::try_context()?;
    context
        .texts
        .get_mut(id)
        .ok_or(TextError::InvalidId(id))?
        .set_text(content);
    Ok(())
}

pub fn set_layout(id: TextId, layout: TextLayout) -> Result<(), TextError> {
    let mut context = context::try_context()?;
    context
        .texts
        .get_mut(id)
        .ok_or(TextError::InvalidId(id))?
        .set_layout(layout)
}

pub fn remove(id: TextId) -> Result<(), TextError> {
    context::try_context()?
        .texts
        .remove(id)
        .ok_or(TextError::InvalidId(id))?;
    Ok(())
}
