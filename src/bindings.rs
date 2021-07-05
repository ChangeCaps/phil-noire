use crate::instance::Instance;
use bytemuck::{bytes_of, cast_slice, Pod};
use std::{collections::HashMap, sync::Arc};
use wgpu::util::DeviceExt;

pub struct UniformBlock(Vec<u8>);

impl UniformBlock {
    #[inline]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[inline]
    pub fn write<T: Pod>(&mut self, uniform: &T) {
        self.0.append(&mut Vec::from(bytes_of(uniform)));
    }

    #[inline]
    pub fn write_slice<T: Pod>(&mut self, slice: &[T]) {
        self.0.append(&mut Vec::from(cast_slice(slice)));
    }

    #[inline]
    pub fn pad(&mut self, amount: usize) {
        self.0.append(&mut vec![0u8; amount]);
    }

    #[inline]
    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}

impl Into<Vec<u8>> for UniformBlock {
    fn into(self) -> Vec<u8> {
        self.finish()
    }
}

pub enum Binding {
    Texture {
        view: Arc<wgpu::TextureView>,
        sample_type: wgpu::TextureSampleType,
    },
    Sampler(Arc<wgpu::Sampler>),
    Uniform {
        buffer: wgpu::Buffer,
        data_len: usize,
    },
}

impl Binding {
    #[inline]
    pub fn ty(&self) -> wgpu::BindingType {
        match self {
            Self::Texture { sample_type, .. } => wgpu::BindingType::Texture {
                sample_type: sample_type.clone(),
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            Self::Sampler(_) => wgpu::BindingType::Sampler {
                filtering: true,
                comparison: false,
            },
            Self::Uniform { .. } => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        }
    }

    #[inline]
    pub fn resource(&self) -> wgpu::BindingResource {
        match self {
            Self::Texture { view, .. } => wgpu::BindingResource::TextureView(view),
            Self::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
            Self::Uniform { buffer, .. } => buffer.as_entire_binding(),
        }
    }

    #[inline]
    pub fn new_texture(
        texture: &Arc<wgpu::TextureView>,
        sample_type: wgpu::TextureSampleType,
    ) -> Self {
        Self::Texture {
            view: texture.clone(),
            sample_type,
        }
    }

    #[inline]
    pub fn new_uniform(instance: &Instance, data: &[u8]) -> Self {
        let buffer = instance
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform buffer"),
                contents: data,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });

        Self::Uniform {
            buffer,
            data_len: data.len(),
        }
    }
}

pub struct BindGroup {
    instance: Instance,
    bindings: HashMap<u32, Binding>,
    bind_group: Option<wgpu::BindGroup>,
}

impl BindGroup {
    pub fn new(instance: &Instance) -> Self {
        Self {
            instance: instance.clone(),
            bindings: HashMap::new(),
            bind_group: None,
        }
    }

    pub fn bind_texture(
        &mut self,
        index: u32,
        texture: &Arc<wgpu::TextureView>,
        sample_type: wgpu::TextureSampleType,
    ) {
        if let Some(binding) = self.bindings.get_mut(&index) {
            match binding {
                Binding::Texture {
                    view,
                    sample_type: ty,
                } => {
                    if Arc::ptr_eq(texture, view) {
                        *ty = sample_type;
                    } else {
                        *binding = Binding::new_texture(texture, sample_type);
                        self.bind_group = None;
                    }
                }
                _ => {
                    *binding = Binding::new_texture(texture, sample_type);
                    self.bind_group = None;
                }
            }
        } else {
            self.bindings
                .insert(index, Binding::new_texture(texture, sample_type));

            self.bind_group = None;
        }
    }

    pub fn bind_sampler(&mut self, index: u32, sampler: &Arc<wgpu::Sampler>) {
        if let Some(binding) = self.bindings.get_mut(&index) {
            match binding {
                Binding::Sampler(this_sampler) => {
                    if !Arc::ptr_eq(sampler, this_sampler) {
                        *binding = Binding::Sampler(sampler.clone());
                        self.bind_group = None;
                    }
                }
                _ => {
                    *binding = Binding::Sampler(sampler.clone());
                    self.bind_group = None;
                }
            }
        } else {
            self.bindings
                .insert(index, Binding::Sampler(sampler.clone()));

            self.bind_group = None;
        }
    }

    pub fn bind_uniform<T: Pod>(&mut self, index: u32, uniform: &T) {
        let data = bytes_of(uniform);

        if let Some(binding) = self.bindings.get_mut(&index) {
            match binding {
                Binding::Uniform { buffer, data_len } => {
                    if data.len() > *data_len {
                        *binding = Binding::new_uniform(&self.instance, data);
                        self.bind_group = None;
                    } else {
                        self.instance.queue.write_buffer(buffer, 0, data);
                    }
                }
                _ => {
                    *binding = Binding::new_uniform(&self.instance, data);
                    self.bind_group = None;
                }
            }
        } else {
            self.bindings
                .insert(index, Binding::new_uniform(&self.instance, data));

            self.bind_group = None;
        }
    }

    pub fn bind_uniform_block(&mut self, index: u32, block: UniformBlock) {
        let data = block.finish();

        if let Some(binding) = self.bindings.get_mut(&index) {
            match binding {
                Binding::Uniform { buffer, data_len } => {
                    if data.len() > *data_len {
                        *binding = Binding::new_uniform(&self.instance, &data);
                        self.bind_group = None;
                    } else {
                        self.instance.queue.write_buffer(buffer, 0, &data);
                    }
                }
                _ => {
                    *binding = Binding::new_uniform(&self.instance, &data);
                    self.bind_group = None;
                }
            }
        } else {
            self.bindings
                .insert(index, Binding::new_uniform(&self.instance, &data));

            self.bind_group = None;
        }
    }

    pub fn inner(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }

    pub fn generate(&mut self) -> &wgpu::BindGroup {
        if let Some(ref bind_group) = self.bind_group {
            bind_group
        } else {
            let entries = self
                .bindings
                .iter()
                .map(|(i, binding)| wgpu::BindGroupLayoutEntry {
                    binding: *i,
                    visibility: wgpu::ShaderStage::all(),
                    ty: binding.ty(),
                    count: None,
                })
                .collect::<Vec<_>>();

            let layout =
                self.instance
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries: &entries,
                    });

            let entries = self
                .bindings
                .iter()
                .map(|(i, binding)| wgpu::BindGroupEntry {
                    binding: *i,
                    resource: binding.resource(),
                })
                .collect::<Vec<_>>();

            let bind_group = self
                .instance
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &layout,
                    entries: &entries,
                });

            self.bind_group = Some(bind_group);

            self.bind_group.as_ref().unwrap()
        }
    }
}
