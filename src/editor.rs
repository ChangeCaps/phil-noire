use crate::{
    instance::Instance,
    node::{drag3, drag_vec3},
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

    pub fn ui(&mut self, world: &mut World) {
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
                });

                ui.collapsing("World", |ui| {
                    for (id, node) in &mut world.nodes {
                        ui.collapsing(id.0, |ui| {
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
