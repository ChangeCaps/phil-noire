use crate::{
    instance::Instance,
    labled,
    node::{drag3, drag_vec3, Node},
    transform::Transform,
    world::World,
};
use egui::*;
use glam::Quat;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct Editor {
    pub input: RawInput,
    pub ctx: CtxRef,
    pub open: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            input: RawInput::default(),
            ctx: CtxRef::default(),
            open: false,
        }
    }

    pub fn ui(&mut self, world: &mut World, loaded_world: &str) {
        if self.ctx.input().key_pressed(Key::Home) {
            self.open ^= true;
        }

        Window::new("Debug")
            .open(&mut self.open)
            .show(&self.ctx, |ui| {
                ui.collapsing("Render Settings", |ui| {
                    ui.add(
                        Slider::new(&mut world.data.render_settings.bloom, 0.0..=1.0).text("Bloom"),
                    );

                    let mut ambient_color = world.data.render_settings.ambient_color.into();
                    labled!(
                        ui,
                        "Ambient Color",
                        ui.color_edit_button_rgb(&mut ambient_color)
                    );
                    world.data.render_settings.ambient_color = ambient_color.into();

                    labled!(
                        ui,
                        "Ambient Strength",
                        ui.add(
                            DragValue::new(&mut world.data.render_settings.ambient_strength)
                                .speed(0.1)
                        )
                    );
                });

                ui.collapsing("World", |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let world_ron = ron::ser::to_string_pretty(
                                world,
                                ron::ser::PrettyConfig::default(),
                            )
                            .unwrap();

                            std::fs::write(loaded_world, world_ron).unwrap();
                        }

                        if ui.button("Add").clicked() {
                            world.spawn(Node {
                                name: String::from("New Node"),
                                transform: Transform::IDENTITY,
                                components: Vec::new(),
                            });
                        }

                        if ui.button("Validate").clicked() {
                            world.validate_next_node();
                        }
                    });

                    ui.separator();

                    ui.label(format!("Next node id: '[{}]'", world.next_node_id.0));
                    ui.label(format!("Next node validated: '{}'", world.next_node_validated));

                    ui.separator();

                    let mut despawn = Vec::new();

                    for (id, node) in &mut world.nodes {
                        CollapsingHeader::new(format!("[{}]: {}", id.0, node.name))
                            .id_source(id)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("Remove").clicked() {
                                        despawn.push(*id);
                                    }
                                });

                                ui.text_edit_singleline(&mut node.name);

                                drag_vec3(ui, &mut node.transform.translation);

                                let (mut y, mut x, mut z) =
                                    node.transform.rotation.to_euler(glam::EulerRot::YXZ);

                                x = x / std::f32::consts::PI * 180.0;
                                y = y / std::f32::consts::PI * 180.0;
                                z = z / std::f32::consts::PI * 180.0;

                                drag3(ui, &mut x, &mut y, &mut z);

                                x = x / 180.0 * std::f32::consts::PI;
                                y = y / 180.0 * std::f32::consts::PI;
                                z = z / 180.0 * std::f32::consts::PI;

                                node.transform.rotation =
                                    Quat::from_euler(glam::EulerRot::YXZ, y, x, z);

                                drag_vec3(ui, &mut node.transform.scale);

                                ui.separator();
                                for (i, component) in node.components.iter_mut().enumerate() {
                                    ui.collapsing(component.name(), |ui| {
                                        component.ui(ui);
                                    });
                                }
                            });
                    }

                    for id in despawn {
                        world.despawn(&id);
                    }
                });
            });
    }

    pub fn texture(&self, instance: &Instance) -> Arc<wgpu::TextureView> {
        let texture = self.ctx.texture();

        let texture = instance.device.create_texture_with_data(
            &instance.queue,
            &wgpu::TextureDescriptor {
                label: Some("egui texture"),
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                size: wgpu::Extent3d {
                    width: texture.width as u32,
                    height: texture.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            },
            &texture
                .pixels
                .iter()
                .map(|a| vec![255u8, 255, 255, *a])
                .flatten()
                .collect::<Vec<_>>(),
        );

        Arc::new(texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("egui view"),
            format: None,
            dimension: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        }))
    }
}
