use crate::gltf;
use crate::types::{ObjectTypeId, ObjectTypeRegistry, ObjectTypeSpec};
use bevy::prelude::*;
use glam::Vec3;
use serde::Deserialize;

#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct CursorHitRes {
    pub world: Option<Vec3>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ObjectKind(pub ObjectTypeId);

#[derive(Resource)]
pub struct ObjectTypesRes {
    pub registry: ObjectTypeRegistry,
    pub available: Vec<ObjectTypeId>,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct HoveredObjectRes(pub Option<Entity>);

pub fn setup_object_hovered(mut commands: Commands) {
    commands.insert_resource(HoveredObjectRes::default());
}

pub fn setup_object_types(mut commands: Commands) {
    let mut registry = ObjectTypeRegistry::default();
    let mut available = Vec::new();

    for def in load_object_type_defs_from_dir("assets/objects")
        .expect("failed to load object type definitions from assets/objects")
    {
        let bounds = gltf::try_compute_gltf_bounds_in_parent_space(&def.gltf).ok();
        let render_scale = Vec3::new(def.scale.0, def.scale.1, def.scale.2);
        let hover_radius = gltf::compute_hover_radius(bounds, render_scale);

        let id = registry.register(ObjectTypeSpec {
            name: def.name,
            gltf: def.gltf,
            gltf_bounds: bounds,
            render_scale,
            hover_radius,
        });
        available.push(id);
    }

    if available.is_empty() {
        let id = registry.register(ObjectTypeSpec {
            name: "MissingObjectDefs".to_string(),
            gltf: "".to_string(),
            gltf_bounds: None,
            render_scale: Vec3::ONE,
            hover_radius: 1.0,
        });
        available.push(id);
    }

    commands.insert_resource(ObjectTypesRes {
        registry,
        available,
    });
}

pub fn spawn_object(
    commands: &mut Commands,
    types: &ObjectTypeRegistry,
    asset_server: &AssetServer,
    type_id: ObjectTypeId,
    position_world: Vec3,
    yaw: f32,
) -> Option<Entity> {
    let spec = types.get(type_id)?;
    if spec.gltf.trim().is_empty() {
        return None;
    }

    let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));
    let rot = Quat::from_rotation_y(yaw);
    let root_transform = Transform::from_translation(position_world)
        .with_rotation(rot)
        .with_scale(spec.render_scale);

    // Offset only the rendered scene so the logical root stays at the object's center.
    let scene_offset_local = compute_scene_local_offset(spec);

    let root = commands
        .spawn((
            ObjectKind(type_id),
            root_transform,
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                SceneRoot(scene_handle),
                Transform::from_translation(scene_offset_local),
                Visibility::default(),
            ));
        })
        .id();

    Some(root)
}

/// Computes the local translation offset to apply to the rendered scene (as a child)
/// so that the model is visually centered on the parent's origin.
///
/// Parent entity should carry the desired world translation/rotation/scale.
pub fn compute_scene_local_offset(spec: &ObjectTypeSpec) -> Vec3 {
    // Pivot point in scene-local coordinates that we want to align with the parent origin.
    // - X/Z: bounds center so rotations happen around the footprint center.
    // - Y: bounds min so the model's base rests on Y=0 relative to the parent.
    let pivot = spec.gltf_bounds.map_or(Vec3::ZERO, |b| Vec3::new(b.center().x, b.min.y, b.center().z));
    -pivot
}

pub fn can_place_non_overlapping(
    types: &ObjectTypeRegistry,
    new_type: ObjectTypeId,
    position_world: Vec3,
    existing: impl Iterator<Item = (ObjectTypeId, Vec3)>,
) -> bool {
    let Some(new_spec) = types.get(new_type) else {
        return false;
    };

    let new_r = new_spec.hover_radius.max(0.1);

    for (other_type, other_pos) in existing {
        let Some(spec) = types.get(other_type) else {
            continue;
        };

        let other_r = spec.hover_radius.max(0.1);

        if circles_overlap(position_world, new_r, other_pos, other_r) {
            return false;
        }
    }

    true
}

pub fn update_hovered_object(
    hit: Res<CursorHitRes>,
    types: Res<ObjectTypesRes>,
    q_objects: Query<(Entity, &Transform, &ObjectKind)>,
    mut hovered: ResMut<HoveredObjectRes>,
) {
    let Some(world) = hit.world else {
        hovered.0 = None;
        return;
    };

    let mut best: Option<(Entity, f32)> = None;

    for (entity, transform, kind) in q_objects.iter() {
        let Some(spec) = types.registry.get(kind.0) else {
            continue;
        };

        let r = spec.hover_radius.max(0.1);

        if !point_in_circle(world, transform.translation, r) {
            continue;
        }

        let dx = transform.translation.x - world.x;
        let dz = transform.translation.z - world.z;
        let d2 = dx * dx + dz * dz;

        if best.map(|(_, b)| d2 < b).unwrap_or(true) {
            best = Some((entity, d2));
        }
    }

    hovered.0 = best.map(|(e, _)| e);
}

fn point_in_circle(p: Vec3, center: Vec3, radius: f32) -> bool {
    let dx = p.x - center.x;
    let dz = p.z - center.z;
    dx * dx + dz * dz <= radius * radius
}

fn circles_overlap(a: Vec3, ar: f32, b: Vec3, br: f32) -> bool {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    let r = ar + br;
    dx * dx + dz * dz <= r * r
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
