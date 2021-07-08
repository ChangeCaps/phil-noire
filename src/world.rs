use crate::{
    instance::Instance,
    mesh::Mesh,
    node::{Node, NodeId},
    renderer::Frame,
};
use gltf::Gltf;
use image::{EncodableLayout, GenericImageView};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::read_to_string,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};
use wgpu::util::DeviceExt;

pub struct Resources {
    pub instance: Instance,
    pub meshes: HashMap<PathBuf, Mesh>,
    pub textures: HashMap<PathBuf, Arc<wgpu::TextureView>>,
    pub worlds: HashMap<PathBuf, World>,
}

impl Resources {
    pub fn new(instance: &Instance) -> Self {
        Self {
            instance: instance.clone(),
            meshes: HashMap::new(),
            textures: HashMap::new(),
            worlds: HashMap::new(),
        }
    }

    pub fn load_assets(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let dir = std::fs::read_dir(path.as_ref())?;

        for entry in dir {
            let entry = entry?;

            if entry.path().is_dir() {
                self.load_assets(entry.path())?;
            } else {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    match ext.to_str().unwrap().to_lowercase().as_str() {
                        "gltf" => self.load_mesh(path)?,
                        "world" => self.load_world(path)?,
                        "png" => self.load_image(path)?,
                        "jpeg" => self.load_image(path)?,
                        "jpg" => self.load_image(path)?,
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_world(&self, path: impl AsRef<Path>) -> Option<&World> {
        self.worlds.get(path.as_ref())
    }

    pub fn load_world(&mut self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();

        log::debug!("loading world: '{:?}'", path);

        let string = read_to_string(&path)?;
        let world = ron::from_str(&string)?;

        self.worlds.insert(path, world);

        Ok(())
    }

    pub fn get_mesh(&self, path: impl AsRef<Path>) -> Option<&Mesh> {
        self.meshes.get(path.as_ref())
    }

    pub fn load_mesh(&mut self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();

        log::debug!("loading mesh: '{:?}'", path);

        let gltf = Gltf::open(&path)?;
        let mut mesh = Mesh::new(&self.instance);
        mesh.load_gltf(&gltf)?;

        mesh.generate_buffers();

        self.meshes.insert(path, mesh);

        Ok(())
    }

    pub fn get_texture(&self, path: impl AsRef<Path>) -> Option<&Arc<wgpu::TextureView>> {
        self.textures.get(path.as_ref())
    }

    pub fn load_image(&mut self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();

        log::debug!("loading image: '{:?}'", path);

        let png = image::open(&path)?;

        let texture = self.instance.device.create_texture_with_data(
            &self.instance.queue,
            &wgpu::TextureDescriptor {
                label: Some("loaded png"),
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                size: wgpu::Extent3d {
                    width: png.width(),
                    height: png.height(),
                    depth_or_array_layers: 1,
                },
                dimension: wgpu::TextureDimension::D2,
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
            },
            png.to_rgba8().as_bytes(),
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("loaded png view"),
            format: None,
            dimension: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        self.textures.insert(path, Arc::new(view));

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RenderSettings {
    pub bloom: f32,
    pub ambient_color: glam::Vec3,
    pub ambient_strength: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            bloom: 0.1,
            ambient_color: glam::Vec3::ONE,
            ambient_strength: 0.0,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct WorldData {
    pub render_settings: RenderSettings,
    pub player: Option<NodeId>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct World {
    pub data: WorldData,
    pub nodes: HashMap<NodeId, Node>,
    pub next_node_id: NodeId,
    #[serde(skip)]
    pub next_node_validated: bool,
}

impl World {
    pub fn new() -> Self {
        Self {
            data: WorldData::default(),
            nodes: HashMap::new(),
            next_node_id: NodeId(0),
            next_node_validated: true,
        }
    }

    #[inline]
    pub fn validate_next_node(&mut self) {
        if !self.next_node_validated {
            let next_id = self
                .nodes
                .keys()
                .map(|id| *id)
                .max()
                .map(|id| NodeId(id.0 + 1))
                .unwrap_or(NodeId(0)); 

            self.next_node_id = next_id;
            self.next_node_validated = true;
        }
    }

    #[inline]
    pub fn generate_node_id(&mut self) -> NodeId {
        self.validate_next_node();

        let id = self.next_node_id;
        self.next_node_id.0 += 1;
        id
    }

    #[inline]
    pub fn spawn(&mut self, node: Node) -> NodeId {
        let id = self.generate_node_id();

        self.nodes.insert(id, node);

        id
    }

    #[inline]
    pub fn despawn(&mut self, id: &NodeId) {
        self.nodes.remove(id);
    }

    #[inline]
    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    #[inline]
    pub fn update(&mut self, resources: &Resources) {
        let ids = self.nodes.keys().cloned().collect::<Vec<_>>();

        for id in ids {
            let mut node = self.nodes.remove(&id).unwrap();

            node.update(id, resources, self);

            self.nodes.insert(id, node);
        }
    }

    #[inline]
    pub fn render<'a>(&'a mut self, resources: &'a Resources, frame: &mut Frame<'a>) {
        frame.bloom = self.data.render_settings.bloom;
        frame.ambient_color = self.data.render_settings.ambient_color;
        frame.ambient_strength = self.data.render_settings.ambient_strength;

        for (id, node) in &mut self.nodes {
            node.render(*id, resources, frame);
        }
    }
}
