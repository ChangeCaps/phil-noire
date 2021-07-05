use crate::transform::Transform;
use glam::*;

pub struct Camera {
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub transform: Transform,
}

impl Camera {
    #[inline]
    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    #[inline]
    pub fn view_matrix(&self) -> Mat4 {
        self.transform.matrix()
    }

    #[inline]
    pub fn view_proj_matrix(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix().inverse()
    }
}
