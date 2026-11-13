/// `define_render_operation! { Decor { ... } execute(this, pass, frame) { ... } }`
/// defines `DecorOperation`. The body returns `Result<(), PassError>`.
#[macro_export]
macro_rules! define_render_operation {
    (
        $name:ident { $($field:ident: $ty:ty),* $(,)? }
        execute($this:ident, $pass:ident, $frame:ident) $body:block
    ) => {
        paste::paste! {
            pub struct [<$name Operation>] {
                $(pub $field: $ty,)*
            }

            impl $crate::backend::pass::RenderOperation for [<$name Operation>] {
                fn execute(
                    &mut self,
                    $pass: &mut wgpu::RenderPass<'_>,
                    $frame: &mut $crate::backend::frame_data::FrameData,
                ) -> Result<(), $crate::backend::pass::PassError> {
                    let $this = self;
                    $body
                }
            }
        }
    };
}

#[macro_export]
macro_rules! define_compute_operation {
    (
        $name:ident { $($field:ident: $ty:ty),* $(,)? }
        execute($this:ident, $pass:ident, $frame:ident) $body:block
    ) => {
        paste::paste! {
            pub struct [<$name Operation>] {
                $(pub $field: $ty,)*
            }

            impl $crate::backend::pass::ComputeOperation for [<$name Operation>] {
                fn execute(
                    &mut self,
                    $pass: &mut wgpu::ComputePass<'_>,
                    $frame: &mut $crate::backend::frame_data::FrameData,
                ) -> Result<(), $crate::backend::pass::PassError> {
                    let $this = self;
                    $body
                }
            }
        }
    };
}
