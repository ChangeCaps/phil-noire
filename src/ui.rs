use bytemuck::{Pod, Zeroable};
use glam::*;

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct UiVertex {
    pub position: Vec2,
    pub uv: Vec2,
    pub color: Vec4,
}

pub struct UiMesh {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
}

impl UiMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}
