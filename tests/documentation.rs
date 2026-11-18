//! These modules must compile against the actual external public API.
//! GPU constructors are deliberately not invoked by ordinary CPU tests.
#![allow(dead_code)]
#[path = "../doc/examples/effet_custom.rs"]
mod effet_custom;
#[path = "../doc/examples/pipeline_custom.rs"]
mod pipeline_custom;
#[path = "../doc/examples/scene_minimale.rs"]
mod scene_minimale;

#[test]
fn custom_effect_satisfies_the_public_trait() {
    fn accepts_effect<T: velum::effects::RenderEffect>() {}
    accepts_effect::<effet_custom::OverlayEffect>();
}
