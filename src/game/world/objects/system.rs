use bevy::prelude::*;
use glam::Vec3;
use serde::Deserialize;

use crate::game::input::CursorHitRes;
use crate::game::utils::gltf;
use crate::game::world::terrain::types::TerrainConfigRes;

use super::storage::ObjectWorld;
use super::types::{ObjectHandle, ObjectTypeId, ObjectTypeRegistry, ObjectTypeSpec};

#[derive(Resource)]
pub struct ObjectWorldRes(pub(crate) ObjectWorld);

#[derive(Resource)]
pub struct ObjectTypesRes {
    pub registry: ObjectTypeRegistry,
    pub available: Vec<ObjectTypeId>,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct HoveredObjectRes(pub Option<ObjectHandle>);

pub fn setup_object_world(mut commands: Commands) {
    commands.insert_resource(ObjectWorldRes(ObjectWorld::new()));
    commands.insert_resource(HoveredObjectRes::default());
}

pub fn setup_object_types(mut commands: Commands, config: Res<TerrainConfigRes>) {
    let mut registry = ObjectTypeRegistry::default();
    let mut available = Vec::new();

    for def in load_object_type_defs_from_dir("assets/objects")
        .expect("failed to load object type definitions from assets/objects")
    {
        let bounds = gltf::try_compute_gltf_bounds_in_parent_space(&def.gltf).ok();

        let render_scale = Vec3::new(def.scale.0, def.scale.1, def.scale.2);

        let (_unused_scale, render_offset, hover_radius) =
            gltf::compute_render_params(config.0.tile_size, bounds, render_scale);

        let id = registry.register(ObjectTypeSpec {
            name: def.name,
            gltf: def.gltf,
            gltf_bounds: bounds,
            render_scale,
            render_offset,
            hover_radius,
        });
        available.push(id);
    }

    if available.is_empty() {
        // If no files exist, keep behavior deterministic.
        let id = registry.register(ObjectTypeSpec {
            name: "MissingObjectDefs".to_string(),
            gltf: "".to_string(),
            gltf_bounds: None,
            render_scale: Vec3::ONE,
            render_offset: Vec3::ZERO,
            hover_radius: 1.0,
        });
        available.push(id);
    }

    commands.insert_resource(ObjectTypesRes {
        registry,
        available,
    });
}

pub fn update_hovered_object(
    hit: Res<CursorHitRes>,
    objects: Res<ObjectWorldRes>,
    types: Res<ObjectTypesRes>,
    mut hovered: ResMut<HoveredObjectRes>,
) {
    let Some(world) = hit.world else {
        hovered.0 = None;
        return;
    };

    hovered.0 = objects.0.pick_hovered(&types.registry, world);
}

#[derive(Debug, Deserialize)]
struct ObjectTypeDefFile {
    name: String,
    gltf: String,
    #[serde(default = "default_object_scale")]
    scale: Scale3,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct Scale3(pub(crate) f32, pub(crate) f32, pub(crate) f32);

fn default_object_scale() -> Scale3 {
    Scale3(1.0, 1.0, 1.0)
}

fn load_object_type_defs_from_dir(
    dir: impl AsRef<std::path::Path>,
) -> Result<Vec<ObjectTypeDefFile>, String> {
    let dir = dir.as_ref();
    let mut defs = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read object defs dir '{}': {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read object defs dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ron") {
            continue;
        }

        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read object def '{}': {e}", path.display()))?;
        let def: ObjectTypeDefFile = ron::from_str(&text)
            .map_err(|e| format!("failed to parse object def '{}': {e}", path.display()))?;

        if def.name.trim().is_empty() {
            return Err(format!("object def '{}' has empty name", path.display()));
        }
        if def.gltf.trim().is_empty() {
            return Err(format!(
                "object def '{}' has empty gltf path",
                path.display()
            ));
        }
        defs.push(def);
    }

    Ok(defs)
}
