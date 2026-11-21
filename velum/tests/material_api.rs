use velum::backend::{BindingSchema, GroupSchema, bindings};

#[test]
fn schemas_keep_sparse_locations_and_reject_conflicts() {
    let entry = |binding| {
        bindings::buffer_entry(
            binding,
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Uniform,
            16,
            false,
        )
    };
    let schema = velum::binding_schema! { 3 => [entry(9), entry(2)], 0 => [entry(7)] };
    let normalized = schema.normalized().unwrap();
    assert_eq!(
        normalized
            .groups
            .iter()
            .map(|g| g.index)
            .collect::<Vec<_>>(),
        [0, 3]
    );
    assert_eq!(
        normalized.groups[1]
            .entries
            .iter()
            .map(|e| e.binding)
            .collect::<Vec<_>>(),
        [2, 9]
    );
    assert!(
        velum::binding_schema! { 3 => [entry(9), entry(9)] }
            .normalized()
            .is_err()
    );
    assert!(
        BindingSchema {
            groups: vec![
                GroupSchema {
                    index: 0,
                    entries: vec![]
                },
                GroupSchema {
                    index: 0,
                    entries: vec![]
                }
            ]
        }
        .normalized()
        .is_err()
    );
    let mut invisible = entry(1);
    invisible.visibility = wgpu::ShaderStages::empty();
    assert!(
        velum::binding_schema! { 0 => [invisible] }
            .normalized()
            .is_err()
    );
}

#[test]
fn fallible_construction_never_requires_a_default_context() {
    assert!(velum::backend::try_shader(None, "missing GPU", "").is_err());
    assert!(
        velum::backend::buffer::allocate_in::<u32, velum::backend::buffer::Uniform>(None, 1)
            .is_err()
    );
    assert!(velum::try_binding_set! {context: None, group: 3, entries: []}.is_err());
}

#[test]
fn cpu_tessellation_appends_nine_patch_with_the_velum_vertex_format() {
    use velum::geometry::{
        NinePatchMargins, Shape, Tesselator,
        {Mesh, Vertex},
    };
    let mut mesh = Mesh::new();
    let shape = Shape::NinePatch {
        pos: [10., 20.],
        size: [64., 48.],
        texture_size: [32., 32.],
        margins: NinePatchMargins::uniform(4.),
        color: [1.; 4],
    };
    Tesselator::tesselate(&shape, &mut mesh);
    assert_eq!(mesh.vertices().len(), 36);
    assert_eq!(mesh.indices().len(), 54);
    assert!(
        mesh.indices()
            .iter()
            .all(|&i| (i as usize) < mesh.vertices().len())
    );
    assert_eq!(std::mem::size_of::<Vertex>(), 32);
    let previous = mesh.vertices().len();
    Tesselator::tesselate(&shape, &mut mesh);
    assert!(mesh.indices()[54..].iter().all(|&i| i as usize >= previous));
    assert!(
        mesh.vertices()
            .iter()
            .all(|v| v.uv.iter().all(|x| (0.0..=1.0).contains(x)))
    );
}

#[test]
fn report_has_explicit_scope_and_frame_outcomes() {
    use velum::backend::report::{self, ExternalCounters, FrameOutcome};
    report::begin();
    report::record_external(ExternalCounters {
        draw_calls: 3,
        instances: 7,
        uploaded_bytes: 128,
        render_passes: 2,
        ..Default::default()
    });
    report::diagnostic("custom operation", "missing application resource");
    report::abandoned();
    velum::backend::metrics::reset(); // The historical metrics API cannot erase a new report.
    let r = report::snapshot();
    assert_eq!(
        (r.draw_calls, r.instances, r.uploaded_bytes, r.render_passes),
        (3, 7, 128, 2)
    );
    assert_eq!(r.outcome, FrameOutcome::Abandoned);
    assert_eq!(r.diagnostics[0].context, "custom operation");
    assert!(r.buffers.is_none());
    report::begin();
    assert_eq!(report::snapshot().draw_calls, 0);
    assert!(report::snapshot().diagnostics.is_empty());
}
