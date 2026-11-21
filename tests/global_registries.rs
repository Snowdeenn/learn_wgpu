use velum::registries::{compute, pipeline};

// Registration itself does not need a device. These methods must not be called
// by the registry, only by a real compute pass.
struct ApplicationPipeline;

impl compute::ComputePipelineContext for ApplicationPipeline {
    fn pipeline(&self) -> &wgpu::ComputePipeline {
        panic!("no GPU pipeline needed for registry access")
    }

    fn bind_groups(&self) -> &[Option<wgpu::BindGroup>] {
        &[]
    }
}

#[test]
fn application_and_backend_share_the_same_registry() {
    let owner = compute::register(ApplicationPipeline);
    let id = owner.id();
    let shared = owner.share();
    let other = shared.clone();

    assert!(velum::backend::pipeline::compute::get(id).is_some());
    assert!(pipeline::get(id).is_some());
    drop(shared);
    assert!(compute::get(id).is_some());
    drop(other);
    assert!(velum::backend::pipeline::compute::get(id).is_none());
    assert!(compute::get(id).is_none());
}
