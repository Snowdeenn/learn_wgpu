/// Texture category expected by an effect input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectInputKind {
    Color,
    Depth,
}

/// Describes an input; the connected target remains owned by its creator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectInputDesc {
    pub name: &'static str,
    pub required: bool,
    pub kind: EffectInputKind,
}
