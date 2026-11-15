/// CPU metadata for ordering draws, independent of texture-array layers.
///
/// The drawing policy decides which field to use. Equal keys should preserve
/// submission order. This type does not enable sorting by itself.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DrawSortKey {
    /// Explicit application order. Smaller values are drawn first.
    pub draw_order: i32,
    /// View-relative depth: smaller values are nearer the camera.
    ///
    /// Must be finite. In 3D this must be computed for the current view rather
    /// than reused across cameras. This CPU key does not configure GPU depth tests.
    pub depth: f32,
}

/// A phase orders only its own commands; equal keys keep submission order.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortPolicy {
    #[default]
    PreserveOrder,
    DrawOrder,
    FrontToBack,
    BackToFront,
}
