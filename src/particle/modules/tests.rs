use super::*;
use crate::particle::data::{ParticleData, ParticleView};

fn init(module: &impl InitModule, data: &mut ParticleData) {
    let len = data.len();
    module.init(ParticleView::from_masks(
        data,
        0..len,
        module.read_mask(),
        module.write_mask(),
    ));
}

fn update(module: &impl UpdateModule, data: &mut ParticleData, dt: f32) {
    let len = data.len();
    module.update(
        ParticleView::from_masks(data, 0..len, module.read_mask(), module.write_mask()),
        dt,
    );
}

fn render(module: &impl RenderModule, data: &mut ParticleData) -> RenderModuleOutput {
    let len = data.len();
    let mut output = RenderModuleOutput::default();
    module.prepare(
        ParticleView::from_masks(data, 0..len, module.read_mask(), 0),
        &mut output,
    );
    output
}

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

#[test]
fn spawn_shapes_respect_volume_and_surface() {
    let mut data = ParticleData::new(512);
    for region in [SpawnRegion::Volume, SpawnRegion::Surface] {
        init(
            &InitPosition {
                shape: PositionShape::Circle {
                    center: [2.0, -1.0],
                    radius: 3.0,
                },
                region,
            },
            &mut data,
        );
        for p in &data.pos {
            let radius = (p[0] - 2.0).hypot(p[1] + 1.0);
            assert!(radius <= 3.00001);
            if matches!(region, SpawnRegion::Surface) {
                close(radius, 3.0);
            }
        }
        init(
            &InitPosition {
                shape: PositionShape::Rectangle {
                    center: [1.0, 2.0],
                    half_extents: [2.0, 4.0],
                },
                region,
            },
            &mut data,
        );
        for p in &data.pos {
            let x = (p[0] - 1.0).abs();
            let y = (p[1] - 2.0).abs();
            assert!(x <= 2.00001 && y <= 4.00001);
            if matches!(region, SpawnRegion::Surface) {
                assert!((x - 2.0).abs() < 1e-5 || (y - 4.0).abs() < 1e-5);
            }
        }
    }
    init(
        &InitPosition {
            shape: PositionShape::Point([7.0, 9.0]),
            region: SpawnRegion::Volume,
        },
        &mut data,
    );
    assert!(data.pos.iter().all(|p| *p == [7.0, 9.0]));
    init(
        &InitPosition {
            shape: PositionShape::Rectangle {
                center: [7.0, 9.0],
                half_extents: [0.0; 2],
            },
            region: SpawnRegion::Surface,
        },
        &mut data,
    );
    assert!(data.pos.iter().all(|p| *p == [7.0, 9.0]));
}

#[test]
fn cone_velocity_stays_inside_sector_and_speed_bounds() {
    let mut data = ParticleData::new(512);
    init(
        &ConeVelocity {
            direction: 0.0,
            spread: 0.3,
            min_speed: 2.0,
            max_speed: 5.0,
        },
        &mut data,
    );
    for v in &data.velocity {
        assert!((2.0 - 1e-5..=5.0 + 1e-5).contains(&v[0].hypot(v[1])));
        assert!(v[1].atan2(v[0]).abs() <= 0.30001);
    }
    init(
        &ConeVelocity {
            direction: 0.0,
            spread: 0.0,
            min_speed: 2.0,
            max_speed: 2.0,
        },
        &mut data,
    );
    assert!(data.velocity.iter().all(|v| *v == [2.0, 0.0]));
}

#[test]
fn radial_velocity_handles_the_center_without_nan() {
    let mut data = ParticleData::new(2);
    data.pos.copy_from_slice(&[[4.0, 6.0], [1.0, 2.0]]);
    init(
        &RadialVelocity {
            center: [1.0, 2.0],
            min_speed: 5.0,
            max_speed: 5.0,
        },
        &mut data,
    );
    assert_eq!(data.velocity, [[3.0, 4.0], [0.0, 0.0]]);
}

#[test]
fn gravity_then_integration_and_rotation_follow_dt() {
    let mut data = ParticleData::new(1);
    update(
        &Gravity {
            acceleration: [0.0, -10.0],
        },
        &mut data,
        0.5,
    );
    update(&IntegratePosition, &mut data, 0.5);
    assert_eq!(data.velocity[0], [0.0, -5.0]);
    assert_eq!(data.pos[0], [0.0, -2.5]);
    init(&InitRotation { min: 1.0, max: 1.0 }, &mut data);
    init(
        &InitAngularVelocity {
            min: -2.0,
            max: -2.0,
        },
        &mut data,
    );
    update(&IntegrateRotation, &mut data, 0.25);
    close(data.rotation[0], 0.5);
}

#[test]
fn drag_is_independent_of_step_partition_and_never_reverses_velocity() {
    let mut whole = ParticleData::new(1);
    let mut split = ParticleData::new(1);
    whole.velocity[0] = [10.0, -8.0];
    split.velocity[0] = whole.velocity[0];
    let drag = Drag { coefficient: 2.0 };
    update(&drag, &mut whole, 1.0);
    for _ in 0..10 {
        update(&drag, &mut split, 0.1);
    }
    close(whole.velocity[0][0], split.velocity[0][0]);
    close(whole.velocity[0][1], split.velocity[0][1]);
    update(&drag, &mut whole, 100.0);
    assert!(whole.velocity[0][0] >= 0.0 && whole.velocity[0][1] <= 0.0);
}

#[test]
fn attractor_handles_its_center_and_noise_is_spatially_continuous() {
    let mut data = ParticleData::new(3);
    data.pos
        .copy_from_slice(&[[0.0, 0.0], [3.0, 4.0], [3.00001, 4.00001]]);
    update(
        &Attractor {
            point: [0.0; 2],
            strength: 5.0,
        },
        &mut data,
        1.0,
    );
    assert_eq!(data.velocity[0], [0.0; 2]);
    assert_eq!(data.velocity[1], [-3.0, -4.0]);
    data.velocity.fill([0.0; 2]);
    let noise = NoiseForce {
        strength: 1.0,
        frequency: 1.0,
        seed: 42,
    };
    update(&noise, &mut data, 1.0);
    let first = data.velocity.clone();
    data.velocity.fill([0.0; 2]);
    update(&noise, &mut data, 1.0);
    assert_eq!(first, data.velocity);
    for axis in 0..2 {
        close(data.velocity[1][axis], data.velocity[2][axis]);
        assert!(
            data.velocity
                .iter()
                .all(|v| (-1.0..=1.0).contains(&v[axis]))
        );
    }
}

#[test]
fn plane_collision_projects_bounces_and_only_friction_on_impact() {
    let mut data = ParticleData::new(3);
    data.pos
        .copy_from_slice(&[[0.0, -2.0], [0.0, -1.0], [0.0, 1.0]]);
    data.velocity
        .copy_from_slice(&[[4.0, -10.0], [4.0, 2.0], [4.0, -10.0]]);
    update(
        &PlaneCollision {
            point: [0.0; 2],
            normal: [0.0, 2.0],
            restitution: 0.5,
            friction: 0.25,
        },
        &mut data,
        1.0,
    );
    assert_eq!(data.pos, [[0.0, 0.0], [0.0, 0.0], [0.0, 1.0]]);
    assert_eq!(data.velocity, [[3.0, 5.0], [4.0, 2.0], [4.0, -10.0]]);
}

#[test]
fn lifetime_curves_clamp_and_do_not_accumulate() {
    let mut data = ParticleData::new(4);
    data.curr_lifetime.copy_from_slice(&[-1.0, 1.0, 3.0, 0.0]);
    data.lifetime.copy_from_slice(&[2.0, 2.0, 2.0, 0.0]);
    let color = ColorOverLifetime {
        start: [1.0; 4],
        end: [0.0; 4],
    };
    let scale = ScaleOverLifetime {
        start: [2.0, 4.0],
        end: [0.0, 0.0],
    };
    for _ in 0..2 {
        update(&color, &mut data, 0.0);
        update(&scale, &mut data, 0.0);
        assert_eq!(data.color, [[1.0; 4], [0.5; 4], [0.0; 4], [0.0; 4]]);
        assert_eq!(data.scale, [[2.0, 4.0], [1.0, 2.0], [0.0; 2], [0.0; 2]]);
    }
}

#[test]
fn animation_selects_layers_and_handles_empty_ranges() {
    let mut data = ParticleData::new(4);
    data.curr_lifetime.copy_from_slice(&[0.0, 0.5, 1.0, 2.0]);
    data.lifetime.fill(1.0);
    data.rotation.fill(0.7);
    let mut module = RenderAnimatedSprite {
        texture: Id::zero(),
        indices: 0..6,
        layers: 4..8,
    };
    let output = render(&module, &mut data);
    assert_eq!(
        output
            .sprite
            .iter()
            .map(|s| s.render.layer)
            .collect::<Vec<_>>(),
        [4, 6, 7, 7]
    );
    for instance in output.sprite {
        close(instance.render.rotation, 0.7);
        let Sprite::Array { layer, .. } = instance.sprite else {
            panic!("expected array sprite");
        };
        assert_eq!(layer, instance.render.layer);
    }
    module.layers = 4..4;
    assert!(render(&module, &mut data).sprite.is_empty());
}

#[test]
fn rotation_adapter_preserves_existing_sprite_output_except_angle() {
    let mut data = ParticleData::new(1);
    data.rotation[0] = 1.2;
    data.pos[0] = [10.0, 20.0];
    let module = WithRotation(RenderSprite {
        texture: Id::zero(),
        indices: 0..6,
    });
    let output = render(&module, &mut data);
    assert_eq!(output.sprite[0].render.offset, [10.0, 20.0]);
    close(output.sprite[0].render.rotation, 1.2);
}

#[test]
fn trail_uses_recent_segments_with_correct_midpoint_length_and_angle() {
    let mut data = ParticleData::new(1);
    data.trail[0].extend([[0.0, 0.0], [3.0, 0.0], [3.0, 4.0]]);
    let module = RenderTrail {
        texture: Id::zero(),
        indices: 0..6,
        max_points: 2,
        width: 0.5,
    };
    let output = render(&module, &mut data);
    assert_eq!(output.sprite.len(), 1);
    assert_eq!(output.sprite[0].render.offset, [3.0, 2.0]);
    assert_eq!(output.sprite[0].render.scale, [4.0, 0.5]);
    close(
        output.sprite[0].render.rotation,
        std::f32::consts::FRAC_PI_2,
    );
    assert_eq!(render(&module, &mut data).sprite.len(), 1);
    assert_eq!(data.trail[0].len(), 3);
}

#[test]
fn resized_particle_data_exposes_new_channels() {
    let mut data = ParticleData::new(1);
    data.resize(3);
    init(&InitRotation { min: 2.0, max: 2.0 }, &mut data);
    assert_eq!(data.rotation, [2.0; 3]);
    assert_eq!(data.angular_velocity.len(), 3);
    assert_eq!(data.trail.len(), 3);
    data.resize(0);
    assert!(data.rotation.is_empty() && data.angular_velocity.is_empty() && data.trail.is_empty());
}

#[test]
fn particle_shaders_validate_and_keep_the_instance_layout() {
    for source in [
        include_str!("../../../assets/shaders/texture.wgsl"),
        include_str!("../../../assets/shaders/texture_array.wgsl"),
        include_str!("../../../assets/shaders/mesh.wgsl"),
    ] {
        let module = wgpu::naga::front::wgsl::parse_str(source).expect("valid WGSL");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("valid shader");
        let instance = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("Dummy"))
            .unwrap()
            .1;
        let wgpu::naga::TypeInner::Struct { span, ref members } = instance.inner else {
            panic!("expected struct");
        };
        assert_eq!(span as usize, std::mem::size_of::<RenderInstance>());
        assert_eq!(span, 64);
        for (name, offset) in [
            ("rotation", std::mem::offset_of!(RenderInstance, rotation)),
            ("_padding1", std::mem::offset_of!(RenderInstance, _padding)),
            ("uv_min", std::mem::offset_of!(RenderInstance, uv_min)),
            ("uv_max", std::mem::offset_of!(RenderInstance, uv_max)),
        ] {
            let member = members
                .iter()
                .find(|member| member.name.as_deref() == Some(name))
                .unwrap();
            assert_eq!(member.offset as usize, offset);
            if name == "rotation" {
                assert!(matches!(
                    module.types[member.ty].inner,
                    wgpu::naga::TypeInner::Scalar(wgpu::naga::Scalar {
                        kind: wgpu::naga::ScalarKind::Float,
                        width: 4,
                    })
                ));
            }
        }
    }
}
