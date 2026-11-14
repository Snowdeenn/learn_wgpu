use super::resources::{
    CircleMesh, Fps, FrameTimings, RectMesh, ScenePipeline, SceneRendering, TriangleMesh,
};
use crate::core::Resources;
use crate::debug::DebugManager;

fn metrics_panel(dm: &DebugManager, rs: &Resources) {
    dm.create_widget("Métriques")
        .window()
        .size([460.0, 540.0])
        .show(|w| {
            let fps = rs.read_resource::<Fps>();
            let mut timings = rs.write_resource::<FrameTimings>();
            w.metric("FPS lissés", &fps);
            w.ui.separator();
            for (label, timing) in [
                ("Simulation particules", &timings.simulation),
                ("Préparation particules", &timings.preparation),
                ("Encodage des passes", &timings.render_encoding),
                ("Acquisition de l'image", &timings.surface_acquire),
            ] {
                w.metric(label, format!("{:.3} ms", timing.average_ms));
            }
            w.ui.small("Temps CPU lissés (10 % de la nouvelle mesure).");
            w.ui.checkbox(&mut timings.measure_gpu_wait, "Mesurer l'attente de fin GPU");
            if timings.measure_gpu_wait {
                w.metric("Attente CPU de fin GPU", format!("{:.3} ms", timings.gpu_wait.average_ms));
            }
            w.ui.small("Cette mesure synchronise CPU/GPU et peut réduire les FPS. Ce n'est pas la durée GPU par timestamps.");
            w.ui.separator();
            const MIB: f64 = 1024.0 * 1024.0;
            w.metric("Instances GPU : capacité réservée", format!("{:.3} Mio", timings.instance_reserved_bytes as f64 / MIB));
            w.metric("Instances envoyées / frame", timings.render.uploaded_bytes / size_of::<crate::backend::RenderInstance>() as u64);
            w.metric("Uploads instances / frame", format!("{:.3} Mio", timings.render.uploaded_bytes as f64 / MIB));
            w.metric("Allocations instances conservées", timings.instance_slots);
            w.metric("Allocations créées / frame", timings.render.instance_allocations);
            w.metric("Batches avant découpage / frame", timings.render.batches);
            w.metric("Draw calls / frame", timings.render.draw_calls);
            w.ui.small("Compteurs de la frame précédente. Draw calls : géométrie et effets, hors texte et egui.");
            w.ui.small("Mémoire : allocations GPU du cache d'instances, hors espaces libres des pages et données CPU.");
        });
}

pub fn run(dm: &DebugManager, rs: &Resources) {
    metrics_panel(dm, rs);

    dm.create_widget("Contrôles")
        .window()
        .size([380.0, 560.0])
        .show(|w| {
            w.create_widget("Rendu de la scène")
                .header(true)
                .show(|w| {
                    let mut selected = *rs.read_resource::<ScenePipeline>();
                    w.ui.horizontal(|ui| {
                        ui.radio_value(&mut selected, ScenePipeline::Mesh, "Formes");
                        ui.radio_value(&mut selected, ScenePipeline::Texture, "Textures");
                    });
                    *rs.write_resource::<ScenePipeline>() = selected;

                    if selected == ScenePipeline::Mesh {
                        w.ui.label("Forme des objets");
                        let mut chosen = None;
                        w.ui.horizontal(|ui| {
                            if ui.button("Triangle").clicked() {
                                chosen = Some(**rs.read_resource::<TriangleMesh>());
                            }
                            if ui.button("Rectangle").clicked() {
                                chosen = Some(**rs.read_resource::<RectMesh>());
                            }
                            if ui.button("Cercle").clicked() {
                                chosen = Some(**rs.read_resource::<CircleMesh>());
                            }
                        });
                        if let Some(mesh) = chosen {
                            for instance in rs.write_resource::<SceneRendering>().instances_mut().iter_mut() {
                                instance.mesh = mesh;
                            }
                        }
                    }
                });

            w.create_widget("Flou des particules")
                .header(true)
                .show(|w| {
                    let mut rendering = rs.write_resource::<SceneRendering>();
                    let settings = &mut rendering.blur.settings;
                    w.ui.add(egui::Slider::new(&mut settings.spacing, 0.0..=4.0)
                        .text("Espacement (pixels)"));
                    w.ui.add(egui::Slider::new(&mut settings.radius, 0..=crate::effects::blur::MAX_BLUR_RADIUS as u32)
                        .text("Rayon (voisins par côté)"));
                    w.ui.label(format!("{} échantillons par pixel et par passe", 2 * settings.radius + 1));
                    w.ui.checkbox(&mut settings.manual, "Poids manuels");
                    w.ui.add_enabled_ui(!settings.manual, |ui| {
                        ui.add(egui::Slider::new(&mut settings.sigma, 0.05..=16.0)
                            .logarithmic(true).text("Sigma (étalement gaussien)"));
                    });
                    w.ui.checkbox(&mut settings.normalize, "Normaliser les poids");
                    if !settings.normalize {
                        w.ui.small("Sans normalisation, la luminosité et l'opacité changent aussi.");
                    }
                    w.ui.small("Rayon ou espacement à 0 : aucun étalement. Chaque poids voisin est appliqué des deux côtés.");
                    if settings.manual && w.button("Charger les poids gaussiens depuis sigma").clicked() {
                        settings.generate_gaussian();
                    }
                    w.ui.collapsing("Poids du noyau", |ui| {
                        let effective = settings.params([1.0, 0.0]);
                        for distance in 0..=settings.radius as usize {
                            ui.horizontal(|ui| {
                                ui.label(if distance == 0 { "Centre".to_owned() } else { format!("±{distance}") });
                                if settings.manual {
                                    ui.add(egui::DragValue::new(&mut settings.weights[distance])
                                        .speed(0.005).range(0.0..=10.0));
                                }
                                ui.label(format!("Poids appliqué : {:.5}", effective.weights[distance][0]));
                            });
                        }
                    });
                    if w.button("Réinitialiser le flou").clicked() {
                        *settings = Default::default();
                    }
                });

            w.create_widget("Objets de la scène")
                .header(true)
                .show(|w| {
                    let mut rendering = rs.write_resource::<SceneRendering>();
                    let instances = rendering.instances_mut();
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .show(w.ui, |ui| {
                            for (index, instance) in instances.iter_mut().enumerate() {
                                ui.push_id(index, |ui| {
                                    ui.collapsing(format!("Instance {index}"), |ui| {
                                        ui.label("Position");
                                        ui.add(egui::Slider::new(&mut instance.offset[0], -300.0..=300.0).text("X"));
                                        ui.add(egui::Slider::new(&mut instance.offset[1], -300.0..=300.0).text("Y"));
                                        ui.label("Taille");
                                        ui.add(egui::Slider::new(&mut instance.scale[0], 0.0..=200.0).text("Largeur"));
                                        ui.add(egui::Slider::new(&mut instance.scale[1], 0.0..=200.0).text("Hauteur"));
                                    });
                                });
                            }
                        });
                });
        });
}
