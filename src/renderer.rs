use crate::{
    bindings::{BindGroup, UniformBlock},
    differed::*,
    instance::Instance,
    mesh::Mesh,
    ui::{UiMesh, UiVertex},
    ui_pipelines::ui_pipeline,
};
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::util::DeviceExt;

fn render_texture(
    instance: &Instance,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> Arc<wgpu::TextureView> {
    let texture = instance.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        dimension: wgpu::TextureDimension::D2,
        format,
        mip_level_count: 1,
        sample_count: 1,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
    });

    Arc::new(texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        aspect: wgpu::TextureAspect::All,
        format: None,
        dimension: None,
        array_layer_count: None,
        base_array_layer: 0,
        mip_level_count: None,
        base_mip_level: 0,
    }))
}

pub struct GBuffer {
    pub sampler: Arc<wgpu::Sampler>,
    pub shadow: Arc<wgpu::TextureView>,
    pub depth: Arc<wgpu::TextureView>,
    pub position: Arc<wgpu::TextureView>,
    pub normal: Arc<wgpu::TextureView>,
    pub albedo: Arc<wgpu::TextureView>,
    pub emission: Arc<wgpu::TextureView>,
    pub bloom: Arc<wgpu::TextureView>,
    pub light: Arc<wgpu::TextureView>,
}

impl GBuffer {
    pub fn new(instance: &Instance, width: u32, height: u32) -> Self {
        let sampler = instance.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("GBuffer sampler"),
            ..Default::default()
        });

        Self {
            sampler: Arc::new(sampler),
            shadow: render_texture(instance, wgpu::TextureFormat::Depth32Float, width, height),
            depth: render_texture(instance, wgpu::TextureFormat::Depth32Float, width, height),
            position: render_texture(instance, wgpu::TextureFormat::Rgba32Float, width, height),
            normal: render_texture(instance, wgpu::TextureFormat::Rgba32Float, width, height),
            albedo: render_texture(instance, wgpu::TextureFormat::Rgba8UnormSrgb, width, height),
            emission: render_texture(instance, wgpu::TextureFormat::Rgba8UnormSrgb, width, height),
            bloom: render_texture(instance, wgpu::TextureFormat::Rgba8UnormSrgb, width, height),
            light: render_texture(instance, wgpu::TextureFormat::Rgba8UnormSrgb, width, height),
        }
    }
}

pub struct RenderPipelines {
    pub shadow: wgpu::RenderPipeline,
    pub separate: wgpu::RenderPipeline,
    pub light: wgpu::RenderPipeline,
    pub bloom: wgpu::RenderPipeline,
    pub combine: wgpu::RenderPipeline,
    pub ui: wgpu::RenderPipeline,
}

impl RenderPipelines {
    pub fn new(instance: &Instance, sc_format: wgpu::TextureFormat) -> Self {
        Self {
            shadow: shadow_pipeline(instance),
            separate: separate_pipeline(instance),
            light: light_pipeline(instance),
            bloom: bloom_pipeline(instance),
            combine: combine_pipeline(instance, sc_format),
            ui: ui_pipeline(instance, sc_format),
        }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Zeroable, Pod, Serialize, Deserialize)]
pub struct DirectionalLight {
    pub direction: Vec3,
    #[serde(skip)]
    pub _pad0: f32,
    pub color: Vec3,
    pub strength: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod, Serialize, Deserialize)]
pub struct PbrMaterial {
    pub albedo: Vec3,
    #[serde(skip)]
    pub _pad0: f32,
    pub emission: Vec3,
    pub specular_bloom: f32,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            albedo: Vec3::ONE,
            _pad0: 0.0,
            emission: Vec3::ZERO,
            specular_bloom: 0.02,
        }
    }
}

pub enum Renderable<'a> {
    Mesh {
        vertex_buffer: &'a wgpu::Buffer,
        index_buffer: &'a wgpu::Buffer,
        indices: u32,
        material: &'a PbrMaterial,
        transform: Mat4,
    },
}

pub enum UiRenderable<'a> {
    Mesh {
        vertices: &'a [UiVertex],
        indices: &'a [u32],
        texture: &'a Arc<wgpu::TextureView>,
    },
}

pub struct Frame<'a> {
    renderables: Vec<Renderable<'a>>,
    ui_renderables: Vec<UiRenderable<'a>>,
    directional_lights: Vec<DirectionalLight>,
    pub aspect: f32,
    pub camera_matrix: Mat4,
    pub camera_position: Vec3,
    pub bloom: f32,
}

impl<'a> Frame<'a> {
    #[inline]
    pub fn new() -> Self {
        Self {
            renderables: Vec::new(),
            ui_renderables: Vec::new(),
            directional_lights: Vec::new(),
            aspect: 0.0,
            camera_matrix: Mat4::ZERO,
            camera_position: Vec3::ZERO,
            bloom: 0.0,
        }
    }

    #[inline]
    pub fn add_directional_light(&mut self, light: DirectionalLight) {
        self.directional_lights.push(light);
    }

    #[inline]
    pub fn render_ui_mesh(&mut self, mesh: &'a UiMesh, texture: &'a Arc<wgpu::TextureView>) {
        self.ui_renderables.push(UiRenderable::Mesh {
            vertices: &mesh.vertices,
            indices: &mesh.indices,
            texture,
        });
    }

    #[inline]
    pub fn render_mesh(&mut self, mesh: &'a Mesh, material: &'a PbrMaterial, transform: Mat4) {
        let indices = mesh.len_indices();
        let (vertex_buffer, index_buffer) = mesh.get_buffers().expect("mesh buffers don't exist");

        self.renderables.push(Renderable::Mesh {
            vertex_buffer,
            index_buffer,
            indices,
            material,
            transform,
        });
    }
}

pub struct UiData {
    pub bindings: BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

impl UiData {
    pub fn new(
        instance: &Instance,
        vertices: &[UiVertex],
        indices: &[u32],
        texture: &Arc<wgpu::TextureView>,
        width: u32,
        height: u32,
    ) -> Self {
        let mut bindings = BindGroup::new(instance);

        bindings.bind_uniform(0, &Vec2::new(width as f32, height as f32));
        bindings.bind_texture(
            1,
            texture,
            wgpu::TextureSampleType::Float { filterable: true },
        );

        bindings.generate();

        let vertex_buffer = instance
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui vertex buffer"),
                contents: cast_slice(vertices),
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            });

        let index_buffer = instance
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui vertex buffer"),
                contents: cast_slice(indices),
                usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            });

        Self {
            bindings,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn update(
        &mut self,
        instance: &Instance,
        vertices: &[UiVertex],
        indices: &[u32],
        texture: &Arc<wgpu::TextureView>,
        width: u32,
        height: u32,
    ) {
        self.bindings
            .bind_uniform(0, &Vec2::new(width as f32, height as f32));
        self.bindings.bind_texture(
            1,
            texture,
            wgpu::TextureSampleType::Float { filterable: true },
        );

        self.bindings.generate();

        self.vertex_buffer =
            instance
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ui vertex buffer"),
                    contents: cast_slice(vertices),
                    usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                });

        self.index_buffer = instance
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui vertex buffer"),
                contents: cast_slice(indices),
                usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            });
    }
}

pub struct Renderer {
    pub g_buffer: GBuffer,
    pub pipelines: RenderPipelines,
    pub light_uniform_bindings: BindGroup,
    pub light_texture_bindings: BindGroup,
    pub bloom_uniform_bindings: BindGroup,
    pub bloom_h_uniform_bindings: BindGroup,
    pub bloom_texture_bindings: BindGroup,
    pub bloom_h_texture_bindings: BindGroup,
    pub combine_texture_bindings: BindGroup,
    pub sampler_bindings: BindGroup,
    pub mesh_bindings: Vec<BindGroup>,
    pub ui_data: Vec<UiData>,
    pub width: u32,
    pub height: u32,
}

impl Renderer {
    pub fn new(
        instance: &Instance,
        sc_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            g_buffer: GBuffer::new(instance, width, height),
            pipelines: RenderPipelines::new(instance, sc_format),
            light_uniform_bindings: BindGroup::new(instance),
            light_texture_bindings: BindGroup::new(instance),
            bloom_uniform_bindings: BindGroup::new(instance),
            bloom_h_uniform_bindings: BindGroup::new(instance),
            bloom_texture_bindings: BindGroup::new(instance),
            bloom_h_texture_bindings: BindGroup::new(instance),
            combine_texture_bindings: BindGroup::new(instance),
            sampler_bindings: BindGroup::new(instance),
            mesh_bindings: Vec::new(),
            ui_data: Vec::new(),
            width,
            height,
        }
    }

    pub fn resize(&mut self, instance: &Instance, width: u32, height: u32) {
        self.g_buffer = GBuffer::new(instance, width, height);
        self.width = width;
        self.height = height;
    }

    pub fn render_frame(
        &mut self,
        instance: &Instance,
        target: &wgpu::TextureView,
        frame: Frame<'_>,
    ) {
        let shadow_pass = wgpu::RenderPassDescriptor {
            label: Some("shadow pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.g_buffer.shadow,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        };

        let separate_pass = wgpu::RenderPassDescriptor {
            label: Some("separate pass"),
            color_attachments: &[
                wgpu::RenderPassColorAttachment {
                    view: &self.g_buffer.position,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                },
                wgpu::RenderPassColorAttachment {
                    view: &self.g_buffer.normal,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                },
                wgpu::RenderPassColorAttachment {
                    view: &self.g_buffer.albedo,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                },
                wgpu::RenderPassColorAttachment {
                    view: &self.g_buffer.emission,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.g_buffer.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        };

        let light_pass = wgpu::RenderPassDescriptor {
            label: Some("light pass"),
            color_attachments: &[
                wgpu::RenderPassColorAttachment {
                    view: &self.g_buffer.light,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                },
                wgpu::RenderPassColorAttachment {
                    view: &self.g_buffer.emission,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        };

        let bloom_pass = wgpu::RenderPassDescriptor {
            label: Some("bloom_pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &self.g_buffer.bloom,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        let bloom_h_pass = wgpu::RenderPassDescriptor {
            label: Some("bloom_pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &self.g_buffer.emission,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        let combine_pass = wgpu::RenderPassDescriptor {
            label: Some("combine pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        let ui_pass = wgpu::RenderPassDescriptor {
            label: Some("ui pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        self.sampler_bindings
            .bind_sampler(0, &self.g_buffer.sampler);

        let mut encoder = instance
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        // separate pass

        let mut render_pass = encoder.begin_render_pass(&separate_pass);

        render_pass.set_pipeline(&self.pipelines.separate);

        let mut mesh_index = 0;

        // prepare bindings
        for renderable in &frame.renderables {
            match *renderable {
                Renderable::Mesh {
                    material,
                    ref transform,
                    ..
                } => {
                    if mesh_index >= self.mesh_bindings.len() {
                        self.mesh_bindings.push(BindGroup::new(instance));
                    }

                    let bind_group = &mut self.mesh_bindings[mesh_index];

                    bind_group.bind_uniform(0, &frame.camera_matrix);
                    bind_group.bind_uniform(1, transform);
                    bind_group.bind_uniform(2, material);

                    bind_group.generate();

                    mesh_index += 1;
                }
            }
        }

        mesh_index = 0;

        // render Renderables
        for renderable in frame.renderables {
            match renderable {
                Renderable::Mesh {
                    vertex_buffer,
                    index_buffer,
                    indices,
                    ..
                } => {
                    let bind_group = &self.mesh_bindings[mesh_index];

                    render_pass.set_bind_group(0, bind_group.inner().unwrap(), &[]);

                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                    render_pass.draw_indexed(0..indices, 0, 0..1);

                    mesh_index += 1;
                }
            }
        }

        drop(render_pass);

        // shadow pass

        let mut render_pass = encoder.begin_render_pass(&shadow_pass);

        render_pass.set_pipeline(&self.pipelines.shadow);

        drop(render_pass);

        // light pass

        let mut directional_lights = UniformBlock::new();

        directional_lights.write(&(frame.directional_lights.len() as u32));
        directional_lights.pad(12);
        directional_lights.write_slice(&frame.directional_lights);

        self.light_uniform_bindings
            .bind_uniform_block(0, directional_lights);
        self.light_uniform_bindings
            .bind_uniform(1, &frame.camera_position);

        self.light_texture_bindings.bind_texture(
            0,
            &self.g_buffer.depth,
            wgpu::TextureSampleType::Depth,
        );
        self.light_texture_bindings.bind_texture(
            1,
            &self.g_buffer.position,
            wgpu::TextureSampleType::Float { filterable: false },
        );
        self.light_texture_bindings.bind_texture(
            2,
            &self.g_buffer.normal,
            wgpu::TextureSampleType::Float { filterable: false },
        );

        let mut render_pass = encoder.begin_render_pass(&light_pass);

        render_pass.set_pipeline(&self.pipelines.light);

        render_pass.set_bind_group(0, self.light_uniform_bindings.generate(), &[]);
        render_pass.set_bind_group(1, self.light_texture_bindings.generate(), &[]);
        render_pass.set_bind_group(2, self.sampler_bindings.generate(), &[]);

        render_pass.draw(0..3, 0..1);

        drop(render_pass);

        // bloom pass

        let mut uniforms = UniformBlock::new();

        let iterations = (self.width.min(self.height) as f32 * frame.bloom).round() as u32;

        uniforms.write(&(false as i32));
        uniforms.write(&iterations);

        self.bloom_uniform_bindings.bind_uniform_block(0, uniforms);

        self.bloom_texture_bindings.bind_texture(
            0,
            &self.g_buffer.emission,
            wgpu::TextureSampleType::Float { filterable: true },
        );

        let mut render_pass = encoder.begin_render_pass(&bloom_pass);

        render_pass.set_pipeline(&self.pipelines.bloom);

        render_pass.set_bind_group(0, self.bloom_uniform_bindings.generate(), &[]);
        render_pass.set_bind_group(1, self.bloom_texture_bindings.generate(), &[]);
        render_pass.set_bind_group(2, self.sampler_bindings.generate(), &[]);

        render_pass.draw(0..3, 0..1);

        drop(render_pass);

        // bloom h pass

        let mut uniforms = UniformBlock::new();

        uniforms.write(&(true as i32));
        uniforms.write(&iterations);

        self.bloom_h_uniform_bindings
            .bind_uniform_block(0, uniforms);

        self.bloom_h_texture_bindings.bind_texture(
            0,
            &self.g_buffer.bloom,
            wgpu::TextureSampleType::Float { filterable: true },
        );

        let mut render_pass = encoder.begin_render_pass(&bloom_h_pass);

        render_pass.set_pipeline(&self.pipelines.bloom);

        render_pass.set_bind_group(0, self.bloom_h_uniform_bindings.generate(), &[]);
        render_pass.set_bind_group(1, self.bloom_h_texture_bindings.generate(), &[]);
        render_pass.set_bind_group(2, self.sampler_bindings.generate(), &[]);

        render_pass.draw(0..3, 0..1);

        drop(render_pass);

        // combine pass

        self.combine_texture_bindings.bind_texture(
            0,
            &self.g_buffer.depth,
            wgpu::TextureSampleType::Depth,
        );
        self.combine_texture_bindings.bind_texture(
            1,
            &self.g_buffer.position,
            wgpu::TextureSampleType::Float { filterable: false },
        );
        self.combine_texture_bindings.bind_texture(
            2,
            &self.g_buffer.normal,
            wgpu::TextureSampleType::Float { filterable: false },
        );
        self.combine_texture_bindings.bind_texture(
            3,
            &self.g_buffer.albedo,
            wgpu::TextureSampleType::Float { filterable: true },
        );
        self.combine_texture_bindings.bind_texture(
            4,
            &self.g_buffer.emission,
            wgpu::TextureSampleType::Float { filterable: true },
        );
        self.combine_texture_bindings.bind_texture(
            5,
            &self.g_buffer.light,
            wgpu::TextureSampleType::Float { filterable: true },
        );

        let mut render_pass = encoder.begin_render_pass(&combine_pass);

        render_pass.set_pipeline(&self.pipelines.combine);

        render_pass.set_bind_group(0, self.combine_texture_bindings.generate(), &[]);
        render_pass.set_bind_group(1, self.sampler_bindings.generate(), &[]);

        render_pass.draw(0..3, 0..1);

        drop(render_pass);

        let mut render_pass = encoder.begin_render_pass(&ui_pass);

        render_pass.set_pipeline(&self.pipelines.ui);

        let mut mesh_index = 0;

        // prepare bindings
        for renderable in &frame.ui_renderables {
            match *renderable {
                UiRenderable::Mesh {
                    vertices,
                    indices,
                    texture,
                    ..
                } => {
                    if mesh_index >= self.ui_data.len() {
                        self.ui_data.push(UiData::new(
                            instance,
                            vertices,
                            indices,
                            texture,
                            self.width,
                            self.height,
                        ));
                    } else {
                        let ui_data = &mut self.ui_data[mesh_index];

                        ui_data.update(
                            instance,
                            vertices,
                            indices,
                            texture,
                            self.width,
                            self.height,
                        );
                    }

                    mesh_index += 1;
                }
            }
        }

        mesh_index = 0;

        // render UiRenderables
        for renderable in frame.ui_renderables {
            match renderable {
                UiRenderable::Mesh { indices, .. } => {
                    let ui_data = &self.ui_data[mesh_index];

                    // TODO
                    //render_pass.set_scissor_rect();

                    render_pass.set_bind_group(0, ui_data.bindings.inner().unwrap(), &[]);
                    render_pass.set_bind_group(1, self.sampler_bindings.inner().unwrap(), &[]);

                    render_pass.set_vertex_buffer(0, ui_data.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        ui_data.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );

                    render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);

                    mesh_index += 1;
                }
            }
        }

        drop(render_pass);

        instance.queue.submit(std::iter::once(encoder.finish()));
    }
}
