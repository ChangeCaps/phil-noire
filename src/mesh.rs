use crate::{
    gltf::{load_buffers, GltfError},
    instance::Instance,
};
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::*;
use gltf::Gltf;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Default, Clone, Copy, Zeroable, Pod)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

#[allow(dead_code)]
impl Vertex {
    #[inline]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_position(Vec3::new(x, y, z))
    }

    #[inline]
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }
}

pub struct Mesh {
    instance: Instance,
    pub vertices: Vec<Vertex>,
    vertex_buffer: Option<wgpu::Buffer>,
    pub indices: Vec<u32>,
    index_buffer: Option<wgpu::Buffer>,
}

impl Clone for Mesh {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance.clone(),
            vertices: self.vertices.clone(),
            vertex_buffer: None,
            indices: self.indices.clone(),
            index_buffer: None,
        }
    }
}

impl Mesh {
    #[inline]
    pub fn new(instance: &Instance) -> Self {
        Self {
            instance: instance.clone(),
            vertices: Vec::new(),
            vertex_buffer: None,
            indices: Vec::new(),
            index_buffer: None,
        }
    }

    pub fn load_gltf(&mut self, gltf: &Gltf) -> anyhow::Result<()> {
        let buffer_data = load_buffers(gltf)?;

        self.vertices.clear();
        self.indices.clear();

        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));

                let positions = reader
                    .read_positions()
                    .ok_or_else(|| GltfError::MissingPositions)?
                    .map(|v| v.into())
                    .collect::<Vec<Vec3>>();

                let normals = reader
                    .read_normals()
                    .ok_or_else(|| GltfError::MissingPositions)?
                    .map(|v| v.into())
                    .collect::<Vec<Vec3>>();

                let uvs = reader
                    .read_tex_coords(0)
                    .ok_or_else(|| GltfError::MissingPositions)?
                    .into_f32()
                    .map(|v| v.into())
                    .collect::<Vec<Vec2>>();

                if let Some(indices) = reader.read_indices() {
                    self.indices.append(
                        &mut indices
                            .into_u32()
                            .map(|i| i + self.vertices.len() as u32)
                            .collect(),
                    );
                }

                let mut vertices: Vec<Vertex> = vec![Default::default(); positions.len()];

                for i in 0..vertices.len() {
                    let vertex = &mut vertices[i];

                    vertex.position = positions[i];
                    vertex.normal = normals[i];
                    vertex.uv = uvs[i];
                }

                self.vertices.append(&mut vertices);
            }
        }

        Ok(())
    }

    #[inline]
    pub fn len_indices(&self) -> u32 {
        self.indices.len() as u32
    }

    #[allow(dead_code)]
    pub fn calculate_normals(&mut self) {
        for vertex in &mut self.vertices {
            vertex.normal = Vec3::ZERO;
        }

        for i in 0..self.indices.len() / 3 {
            let i0 = self.indices[i * 3 + 0] as usize;
            let i1 = self.indices[i * 3 + 1] as usize;
            let i2 = self.indices[i * 3 + 2] as usize;

            let v0 = self.vertices[i0];
            let v1 = self.vertices[i1];
            let v2 = self.vertices[i2];

            let normal = (v2.position - v0.position)
                .cross(v1.position - v0.position)
                .normalize();

            self.vertices[i0].normal += normal;
            self.vertices[i1].normal += normal;
            self.vertices[i2].normal += normal;
        }

        for vertex in &mut self.vertices {
            vertex.normal = vertex.normal.normalize();
        }

        self.write_vertex_buffer();
    }

    pub fn init_vertex_buffer(&mut self) {
        if self.vertex_buffer.is_none() {
            let buffer =
                self.instance
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vertex buffer"),
                        contents: cast_slice(&self.vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    });

            self.vertex_buffer = Some(buffer);
        }
    }

    pub fn write_vertex_buffer(&mut self) {
        if let Some(ref buffer) = self.vertex_buffer {
            self.instance
                .queue
                .write_buffer(&buffer, 0, cast_slice(&self.vertices));
        } else {
            self.init_vertex_buffer();
        }
    }

    pub fn generate_buffers(&mut self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        let vertex_buffer = if let Some(ref buffer) = self.vertex_buffer {
            buffer
        } else {
            self.init_vertex_buffer();

            self.vertex_buffer.as_ref().unwrap()
        };

        let index_buffer = if let Some(ref buffer) = self.index_buffer {
            buffer
        } else {
            let buffer =
                self.instance
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("index buffer"),
                        contents: cast_slice(&self.indices),
                        usage: wgpu::BufferUsage::INDEX,
                    });

            self.index_buffer = Some(buffer);

            self.index_buffer.as_ref().unwrap()
        };

        (vertex_buffer, index_buffer)
    }

    pub fn get_buffers(&self) -> Option<(&wgpu::Buffer, &wgpu::Buffer)> {
        Some((self.vertex_buffer.as_ref()?, self.index_buffer.as_ref()?))
    }
}
