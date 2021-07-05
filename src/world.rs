use crate::{
    instance::Instance,
    mesh::Mesh,
    node::{Node, NodeId},
    renderer::Frame,
};
use gltf::Gltf;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
};

pub struct Resources {
    pub instance: Instance,
    pub meshes: HashMap<PathBuf, Mesh>,
    pub worlds: HashMap<PathBuf, World>,
}

impl Resources {
    pub fn new(instance: &Instance) -> Self {
        Self {
            instance: instance.clone(),
            meshes: HashMap::new(),
            worlds: HashMap::new(),
        }
    }

    pub fn load_assets(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let dir = std::fs::read_dir(path.as_ref())?;

        for entry in dir {
            let entry = entry?;

            if entry.path().is_dir() {
                self.load_assets(path.as_ref())?;
            } else {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    match ext.to_str().unwrap() {
                        "gltf" => self.load_mesh(path)?,
                        "world" => self.load_world(path)?,
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
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RenderSettings {
    pub bloom: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self { bloom: 0.1 }
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
}

impl World {
    pub fn new() -> Self {
        Self {
            data: WorldData::default(),
            nodes: HashMap::new(),
            next_node_id: NodeId(0),
        }
    }

    #[inline]
    pub fn generate_node_id(&mut self) -> NodeId {
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

        for (id, node) in &mut self.nodes {
            node.render(*id, resources, frame);
        }
    }
}
