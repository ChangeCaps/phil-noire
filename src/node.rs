use crate::{
    renderer::{DirectionalLight, Frame, PbrMaterial},
    transform::Transform,
    world::{Resources, World},
};
use egui::*;
use glam::{Vec2, *};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub transform: Transform,
    pub components: Vec<Component>,
}

impl Node {
    #[inline]
    pub fn update(&mut self, id: NodeId, resources: &Resources, world: &mut World) {
        for component in &mut self.components {
            component.update(id, &mut self.transform, resources, world);
        }
    }

    #[inline]
    pub fn render<'a>(&'a mut self, id: NodeId, resources: &'a Resources, frame: &mut Frame<'a>) {
        for component in &mut self.components {
            component.render(id, resources, &self.transform, frame);
        }
    }
}

pub fn drag3(
    ui: &mut egui::Ui,
    x: &mut impl egui::emath::Numeric,
    y: &mut impl egui::emath::Numeric,
    z: &mut impl egui::emath::Numeric,
) {
    ui.columns(3, |columns| {
        columns[0].add(DragValue::new(x));
        columns[1].add(DragValue::new(y));
        columns[2].add(DragValue::new(z));
    });
}

pub fn drag_vec3(ui: &mut egui::Ui, vec3: &mut Vec3) {
    let mut x = vec3.x;
    let mut y = vec3.y;
    let mut z = vec3.z;

    drag3(ui, &mut x, &mut y, &mut z);

    vec3.x = x;
    vec3.y = y;
    vec3.z = z;
}

#[macro_export]
macro_rules! labled {
    ($ui:ident, $label:literal, $add:expr) => {
        $ui.horizontal(|$ui| {
            $ui.label($label);
            $add;
        });
    };
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Component {
    Mesh { mesh: String, material: PbrMaterial },
    DirectionalLight(DirectionalLight),
    Camera { fov: f32, near: f32, far: f32 },
    Player,
    PlayerCamera,
}

impl Component {
    pub const MESH: &'static str = "Mesh";
    pub const DIRECTIONAL_LIGHT: &'static str = "Directional Light";
    pub const CAMERA: &'static str = "Camera";
    pub const PLAYER: &'static str = "Player";
    pub const PLAYER_CAMERA: &'static str = "Player Camera";

    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Mesh { .. } => Self::MESH,
            Self::DirectionalLight { .. } => Self::DIRECTIONAL_LIGHT,
            Self::Camera { .. } => Self::CAMERA,
            Self::Player { .. } => Self::PLAYER,
            Self::PlayerCamera { .. } => Self::PLAYER_CAMERA,
        }
    }

    #[inline]
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        match self {
            Self::Mesh { mesh, material } => {
                ui.text_edit_singleline(mesh);

                let mut albedo = material.albedo.into();
                ui.color_edit_button_rgb(&mut albedo);
                material.albedo = albedo.into();

                let mut emission = material.emission.into();
                ui.color_edit_button_rgb(&mut emission);
                material.emission = emission.into();

                ui.add(Slider::new(&mut material.specular_bloom, 0.0..=1.0).text("Specular Bloom"));
            }
            Self::Camera { fov, near, far } => {
                labled!(ui, "fov", ui.add(DragValue::new(fov)));
                labled!(ui, "near", ui.add(DragValue::new(near)));
                labled!(ui, "far", ui.add(DragValue::new(far)));
            }
            Self::DirectionalLight(light) => {
                labled!(ui, "direction", drag_vec3(ui, &mut light.direction));

                let mut color = light.color.into();
                ui.color_edit_button_rgb(&mut color);
                light.color = color.into();

                labled!(ui, "strength", ui.add(DragValue::new(&mut light.strength)));
            }
            _ => {}
        }
    }

    #[inline]
    pub fn update(
        &mut self,
        id: NodeId,
        transform: &mut Transform,
        resources: &Resources,
        world: &mut World,
    ) {
        match self {
            Self::Player => world.data.player = Some(id),
            Self::PlayerCamera => {
                if let Some(player) = world.data.player {
                    let player = world.node(&player).unwrap();

                    transform.translation =
                        player.transform.translation + Vec3::new(0.0, 20.0, -3.0);

                    transform.look_at(player.transform.translation, Vec3::Z);
                }
            }
            _ => {}
        }
    }

    #[inline]
    pub fn render<'a>(
        &'a mut self,
        id: NodeId,
        resources: &'a Resources,
        transform: &Transform,
        frame: &mut Frame<'a>,
    ) {
        match self {
            Self::Mesh { mesh, material } => {
                if let Some(mesh) = resources.get_mesh(mesh) {
                    frame.render_mesh(mesh, material, transform.matrix())
                }
            }
            Self::DirectionalLight(light) => frame.add_directional_light(*light),
            Self::Camera { fov, near, far } => {
                let proj = Mat4::perspective_rh(
                    *fov / 180.0 * std::f32::consts::PI,
                    frame.aspect,
                    *near,
                    *far,
                );
                let view_proj = proj * transform.matrix().inverse();

                frame.camera_matrix = view_proj;
                frame.camera_position = transform.translation;
            }
            _ => {}
        }
    }
}
