use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use glam::IVec2;

use crate::object_system::{
    FreeformObjectWorldRes, HoveredObjectRes, ObjectTypesRes, PlacementRotationRes,
};
use crate::selection::CursorHitRes;
use crate::terrain_renderer::{LoadedChunkEntities, TerrainWorldRes};
use crate::toolbar::{ToolbarMode, ToolbarState};

#[derive(Resource, Default)]
pub(crate) struct HologramPreviewRes {
    entity: Option<Entity>,
}

#[derive(Resource)]
pub(crate) struct HologramMaterialsRes {
    pub(crate) valid: Handle<StandardMaterial>,
    pub(crate) blocked: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub(crate) struct LoadedObjectChunkEntities {
    pub(crate) entities: std::collections::HashMap<IVec2, Entity>,
}

#[derive(Component)]
pub(crate) struct ObjectChunkRoot {
    coord: IVec2,
}

pub(crate) fn setup_object_renderer(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(LoadedObjectChunkEntities::default());

    let hologram_valid = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.90, 1.00, 0.35),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let hologram_blocked = materials.add(StandardMaterial {
        base_color: Color::srgba(1.00, 0.25, 0.25, 0.35),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(HologramMaterialsRes {
        valid: hologram_valid,
        blocked: hologram_blocked,
    });
    commands.insert_resource(HologramPreviewRes::default());
}

/// Keep object chunk roots in sync with loaded terrain chunks.
pub(crate) fn sync_object_chunk_roots(
    mut commands: Commands,
    terrain: Res<TerrainWorldRes>,
    loaded_terrain: Res<LoadedChunkEntities>,
    mut loaded_objects: ResMut<LoadedObjectChunkEntities>,
    children: Query<&Children>,
) {
    // Despawn roots for terrain chunks that are no longer loaded.
    loaded_objects
        .entities
        .retain(|coord, entity| {
            if loaded_terrain.entities.contains_key(coord) {
                true
            } else {
                if let Ok(kids) = children.get(*entity) {
                    for child in kids.iter() {
                        commands.entity(child).despawn();
                    }
                }
                commands.entity(*entity).despawn();
                false
            }
        });

    // Spawn roots for newly loaded terrain chunks.
    for coord in loaded_terrain.entities.keys().copied() {
        if loaded_objects.entities.contains_key(&coord) {
            continue;
        }

        let origin = terrain.0.chunk_origin_world(coord);
        let entity = commands
            .spawn((
                ObjectChunkRoot { coord },
                Transform::from_translation(Vec3::new(origin.x, 0.0, origin.z)),
                Visibility::default(),
            ))
            .id();

        loaded_objects.entities.insert(coord, entity);
    }
}

/// Rebuild object visuals for dirty chunks only.
pub(crate) fn update_object_chunk_visuals(
    mut commands: Commands,
    terrain: Res<TerrainWorldRes>,
    asset_server: Res<AssetServer>,
    mut objects: ResMut<FreeformObjectWorldRes>,
    types: Res<ObjectTypesRes>,
    loaded_objects: Res<LoadedObjectChunkEntities>,
    roots: Query<(Entity, &ObjectChunkRoot)>,
    children: Query<&Children>,
    all_entities: Query<Entity>,
) {
    let _tile_size = terrain.0.config.tile_size;

    for (root_entity, root) in roots.iter() {
        let chunk_origin = terrain.0.chunk_origin_world(root.coord);

        // Only rebuild if dirty, or if the chunk just got created.
        let dirty = objects.0.chunk_is_dirty(root.coord);
        let is_known_loaded = loaded_objects.entities.get(&root.coord) == Some(&root_entity);
        if !dirty && is_known_loaded {
            continue;
        }

        // Clear existing children.
        if let Ok(kids) = children.get(root_entity) {
            for child in kids.iter() {
                if all_entities.get(child).is_ok() {
                    commands.entity(child).despawn();
                }
            }
        }

        // Spawn one glTF scene per object in this chunk.
        let mut to_spawn = Vec::new();
        for handle in objects.0.iter_objects_in_chunk(root.coord) {
            let Some(instance) = objects.0.get(handle) else {
                continue;
            };

            let Some(spec) = types.registry.get(instance.type_id) else {
                continue;
            };

            if spec.gltf.trim().is_empty() {
                continue;
            }

            let base_h = terrain
                .0
                .sample_height_at(instance.position_world.x, instance.position_world.z);

            let object_center_x = instance.position_world.x;
            let object_center_z = instance.position_world.z;

            // IMPORTANT: spawned as a CHILD of the chunk root.
            // Child transform is local to the root, so convert world->chunk-local.
            let base_local_pos = Vec3::new(
                object_center_x - chunk_origin.x,
                base_h,
                object_center_z - chunk_origin.z,
            );

            // Auto-center + auto-scale the glTF into the tile footprint.
            // Many downloadable models have coordinates in centimeters and far from origin,
            // which can make them appear "invisible" (actually spawned offscreen).

            let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));
            let rot = Quat::from_rotation_y(instance.yaw);
            let rotated_offset = rot * Vec3::new(spec.render_offset.x, spec.render_offset.y, spec.render_offset.z);
            to_spawn.push((
                scene_handle,
                base_local_pos + rotated_offset,
                spec.render_scale,
                rot,
            ));
        }

        commands.entity(root_entity).with_children(|parent| {
            for (scene_handle, pos, scale, rot) in to_spawn.drain(..) {
                parent.spawn((
                    SceneRoot(scene_handle),
                    Transform::from_translation(pos)
                        .with_rotation(rot)
                        .with_scale(scale),
                    Visibility::default(),
                ));
            }
        });

        objects.0.mark_chunk_clean(root.coord);
    }
}

pub(crate) fn update_hologram_preview(
    mut commands: Commands,
    terrain: Res<TerrainWorldRes>,
    asset_server: Res<AssetServer>,
    types: Res<ObjectTypesRes>,
    objects: Res<FreeformObjectWorldRes>,
    toolbar: Res<ToolbarState>,
    hit: Res<CursorHitRes>,
    placement_rot: Res<PlacementRotationRes>,
    hologram_materials: Res<HologramMaterialsRes>,
    mut preview: ResMut<HologramPreviewRes>,
    children: Query<&Children>,
    mut q_materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    // Only show hologram in construction mode with a valid cursor hit.
    let (show, object_to_preview) = match toolbar.mode {
        ToolbarMode::Construct { object } if hit.world.is_some() => (true, Some(object)),
        _ => (false, None),
    };
    if !show {
        if let Some(e) = preview.entity.take() {
            despawn_recursive(&mut commands, &children, e);
        }
        return;
    }

    let Some(object_type) = object_to_preview else {
        return;
    };

    let Some(spec) = types.registry.get(object_type) else {
        return;
    };
    if spec.gltf.trim().is_empty() {
        return;
    }
    let Some(hit_world) = hit.world else {
        return;
    };

    let base_h = terrain.0.sample_height_at(hit_world.x, hit_world.z);
    let rot = Quat::from_rotation_y(placement_rot.yaw);
    let rotated_offset = rot * Vec3::new(spec.render_offset.x, spec.render_offset.y, spec.render_offset.z);

    let pos_world = Vec3::new(hit_world.x, base_h, hit_world.z) + rotated_offset;
    let transform = Transform::from_translation(pos_world)
        .with_rotation(rot)
        .with_scale(spec.render_scale);

    let can_place = objects
        .0
        .can_place_non_overlapping(&types.registry, object_type, hit_world);

    let chosen_material = if can_place {
        &hologram_materials.valid
    } else {
        &hologram_materials.blocked
    };

    let preview_entity = match preview.entity {
        Some(e) => {
            commands.entity(e).insert(transform);
            e
        }
        None => {
            let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));
            let e = commands
                .spawn((SceneRoot(scene_handle), transform, Visibility::default()))
                .id();
            preview.entity = Some(e);
            e
        }
    };

    // Force hologram material on any mesh materials under the preview root.
    // We do this every frame because glTF scenes spawn their mesh children asynchronously.
    apply_hologram_material_recursive(
        &children,
        &mut q_materials,
        preview_entity,
        chosen_material,
        0,
    );
}

fn despawn_recursive(commands: &mut Commands, children: &Query<&Children>, entity: Entity) {
    if let Ok(kids) = children.get(entity) {
        for child in kids.iter() {
            despawn_recursive(commands, children, child);
        }
    }
    commands.entity(entity).despawn();
}

fn apply_hologram_material_recursive(
    children: &Query<&Children>,
    materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    entity: Entity,
    hologram: &Handle<StandardMaterial>,
    depth: usize,
) {
    if depth > 96 {
        return;
    }

    if let Ok(mut mat) = materials.get_mut(entity) {
        mat.0 = hologram.clone();
    }

    if let Ok(kids) = children.get(entity) {
        for child in kids.iter() {
            apply_hologram_material_recursive(children, materials, child, hologram, depth + 1);
        }
    }
}

pub(crate) fn draw_hover_highlight(
    mut gizmos: Gizmos,
    terrain: Res<TerrainWorldRes>,
    hovered: Res<HoveredObjectRes>,
    objects: Res<FreeformObjectWorldRes>,
    types: Res<ObjectTypesRes>,
) {
    let Some(h) = hovered.0 else {
        return;
    };
    let Some(inst) = objects.0.get(h) else {
        return;
    };
    let Some(spec) = types.registry.get(inst.type_id) else {
        return;
    };

    let r = spec.hover_radius.max(0.25);
    let y = terrain
        .0
        .sample_height_at(inst.position_world.x, inst.position_world.z)
        + 0.05;
    let center = Vec3::new(inst.position_world.x, y, inst.position_world.z);

    let segments = 32;
    let mut prev = None;
    for i in 0..=segments {
        let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let p = center + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        if let Some(pr) = prev {
            gizmos.line(pr, p, Color::WHITE);
        }
        prev = Some(p);
    }
}
