use crate::types::{ObjectTypeId, ObjectTypeRegistry, ObjectTypeSpec};
use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use bevy::tasks::futures_lite::future;
use glam::Vec3;

use crate::assets::ObjectTypeDefAsset;
use crate::spatial::SpatialHashGrid;
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct CursorHit {
    pub world: Option<Vec3>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ObjectKind(pub ObjectTypeId);

#[derive(Resource)]
pub struct ObjectTypes {
    pub registry: ObjectTypeRegistry,
    pub available: Vec<ObjectTypeId>,
    pub max_hover_radius: f32,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct HoveredObject(pub Option<Entity>);

pub fn setup_object_hovered(mut commands: Commands) {
    commands.insert_resource(HoveredObject::default());
}

#[derive(Resource)]
pub struct ObjectDefDiscoveryTask(pub Task<Result<Vec<String>, String>>);

#[derive(Resource)]
pub struct ObjectDefHandles {
    pub handles: Vec<Handle<ObjectTypeDefAsset>>,
}

pub fn setup_object_types(mut commands: Commands) {
    let pool = IoTaskPool::get();
    let task = pool.spawn(async move { discover_object_def_asset_paths("assets/objects") });
    commands.insert_resource(ObjectDefDiscoveryTask(task));
}

pub fn finish_object_types_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    defs: Res<Assets<ObjectTypeDefAsset>>,
    task: Option<ResMut<ObjectDefDiscoveryTask>>,
    handles: Option<Res<ObjectDefHandles>>,
) {
    if task.is_some() {
        let Some(mut task) = task else {
            return;
        };

        let Some(result) = future::block_on(future::poll_once(&mut task.0)) else {
            return;
        };

        commands.remove_resource::<ObjectDefDiscoveryTask>();

        match result {
            Ok(paths) => {
                let mut handles = Vec::new();
                for p in paths {
                    handles.push(asset_server.load::<ObjectTypeDefAsset>(p));
                }
                commands.insert_resource(ObjectDefHandles { handles });
            }
            Err(e) => {
                error!("{e}");
                commands.insert_resource(make_missing_object_defs());
            }
        }

        return;
    }

    let Some(handles) = handles else {
        return;
    };

    if handles.handles.is_empty() {
        commands.remove_resource::<ObjectDefHandles>();
        commands.insert_resource(make_missing_object_defs());
        return;
    }

    // Wait until all assets are loaded (or failed).
    for h in &handles.handles {
        if defs.get(h).is_some() {
            continue;
        }

        // If a load failed, don't wait forever.
        if let Some(state) = asset_server.get_load_state(h.id()) {
            match state {
                bevy::asset::LoadState::Failed(_) => {
                    error!("failed to load object def asset");
                    commands.remove_resource::<ObjectDefHandles>();
                    commands.insert_resource(make_missing_object_defs());
                    return;
                }
                _ => {}
            }
        }

        return;
    }

    let mut registry = ObjectTypeRegistry::default();
    let mut available = Vec::new();
    let mut max_hover_radius = 0.0f32;

    for h in &handles.handles {
        let Some(def) = defs.get(h) else {
            continue;
        };

        max_hover_radius = max_hover_radius.max(def.hover_radius.max(0.1));
        let id = registry.register(ObjectTypeSpec {
            name: def.name.clone(),
            gltf: def.gltf.clone(),
            render_scale: def.render_scale,
            hover_radius: def.hover_radius,
            scene_offset_local: def.scene_offset_local,
        });
        available.push(id);
    }

    commands.remove_resource::<ObjectDefHandles>();
    commands.insert_resource(ObjectTypes {
        registry,
        available,
        max_hover_radius,
    });
}

fn make_missing_object_defs() -> ObjectTypes {
    let mut registry = ObjectTypeRegistry::default();
    let id = registry.register(ObjectTypeSpec {
        name: "MissingObjectDefs".to_string(),
        gltf: "".to_string(),
        render_scale: Vec3::ONE,
        hover_radius: 1.0,
        scene_offset_local: Vec3::ZERO,
    });

    ObjectTypes {
        registry,
        available: vec![id],
        max_hover_radius: 1.0,
    }
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

    let root = commands
        .spawn((
            ObjectKind(type_id),
            root_transform,
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                SceneRoot(scene_handle),
                Transform::from_translation(spec.scene_offset_local),
                Visibility::default(),
            ));
        })
        .id();

    Some(root)
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

pub fn can_place_non_overlapping_spatial(
    types: &ObjectTypeRegistry,
    new_type: ObjectTypeId,
    position_world: Vec3,
    grid: &SpatialHashGrid,
    q_objects: &Query<(&Transform, &ObjectKind)>,
) -> bool {
    let Some(new_spec) = types.get(new_type) else {
        return false;
    };

    let new_r = new_spec.hover_radius.max(0.1);
    let candidates = grid.query_candidates(glam::Vec2::new(position_world.x, position_world.z), new_r);

    for e in candidates {
        let Ok((t, k)) = q_objects.get(e) else {
            continue;
        };
        let Some(spec) = types.get(k.0) else {
            continue;
        };
        let other_r = spec.hover_radius.max(0.1);
        if circles_overlap(position_world, new_r, t.translation, other_r) {
            return false;
        }
    }

    true
}

pub fn update_hovered_object(
    hit: Res<CursorHit>,
    types: Option<Res<ObjectTypes>>,
    q_objects: Query<(Entity, &Transform, &ObjectKind)>,
    grid: Res<SpatialHashGrid>,
    mut hovered: ResMut<HoveredObject>,
) {
    let Some(types) = types else {
        hovered.0 = None;
        return;
    };
    let Some(world) = hit.world else {
        hovered.0 = None;
        return;
    };

    let mut best: Option<(Entity, f32)> = None;

    let candidates = grid.query_candidates(glam::Vec2::new(world.x, world.z), types.max_hover_radius);

    for entity in candidates {
        let Ok((_e, transform, kind)) = q_objects.get(entity) else {
            continue;
        };
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

fn discover_object_def_asset_paths(dir: impl AsRef<std::path::Path>) -> Result<Vec<String>, String> {
    let dir = dir.as_ref();
    let mut out = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read object defs dir '{}': {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read object defs dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ron") {
            continue;
        }

        // Convert filesystem path under assets/ into an asset-relative path.
        let rel = path
            .strip_prefix("assets")
            .map_err(|_| format!("object def '{}' is not under assets/", path.display()))?;
        let mut s = rel.to_string_lossy().to_string();
        // Bevy asset paths use '/' even on Windows.
        s = s.replace('\\', "/");
        if s.starts_with('/') {
            s.remove(0);
        }
        out.push(s);
    }

    Ok(out)
}
