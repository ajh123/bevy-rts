use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use glam::IVec2;

use crate::object_system::{ObjectTypesRes, ObjectWorldRes};
use crate::terrain_renderer::{LoadedChunkEntities, TerrainWorldRes};

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
) {
    commands.insert_resource(LoadedObjectChunkEntities::default());
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
    mut objects: ResMut<ObjectWorldRes>,
    types: Res<ObjectTypesRes>,
    loaded_objects: Res<LoadedObjectChunkEntities>,
    roots: Query<(Entity, &ObjectChunkRoot)>,
    children: Query<&Children>,
    all_entities: Query<Entity>,
) {
    let tile_size = terrain.0.config.tile_size;

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

        // Spawn one glTF scene per object origin tile in this chunk.
        let mut to_spawn = Vec::new();
        for handle in objects.0.iter_origin_objects_in_chunk(root.coord) {
            let Some(instance) = objects.0.get(handle) else {
                continue;
            };

            let Some(spec) = types.registry.get(instance.type_id) else {
                continue;
            };

            if spec.gltf.trim().is_empty() {
                continue;
            }

            // Compute a conservative base height using the max height under the footprint.
            // This avoids objects clipping into sloped terrain.
            let base_h = compute_footprint_base_height(&terrain.0, instance.origin_tile, instance.size_tiles);

            let origin_corner_x = instance.origin_tile.x as f32 * tile_size;
            let origin_corner_z = instance.origin_tile.y as f32 * tile_size;
            let object_center_x = origin_corner_x + (instance.size_tiles.x as f32) * tile_size * 0.5;
            let object_center_z = origin_corner_z + (instance.size_tiles.y as f32) * tile_size * 0.5;

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
            let (extra_offset, uniform_scale) = if let Some(bounds) = spec.gltf_bounds {
                let size = bounds.size();
                let size_x = size.x.abs().max(1e-3);
                let size_z = size.z.abs().max(1e-3);
                let desired_x = (instance.size_tiles.x as f32) * tile_size;
                let desired_z = (instance.size_tiles.y as f32) * tile_size;
                let s = (desired_x / size_x).min(desired_z / size_z).clamp(0.0001, 1000.0);

                let center = bounds.center();
                let min_y = bounds.min.y;

                let offset = Vec3::new(-center.x * s, -min_y * s, -center.z * s);
                (offset, s)
            } else {
                (Vec3::ZERO, 1.0)
            };

            let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));
            to_spawn.push((scene_handle, base_local_pos + extra_offset, uniform_scale));
        }

        commands.entity(root_entity).with_children(|parent| {
            for (scene_handle, pos, scale) in to_spawn.drain(..) {
                parent.spawn((
                    SceneRoot(scene_handle),
                    Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
                    Visibility::default(),
                ));
            }
        });

        objects.0.mark_chunk_clean(root.coord);
    }
}

fn compute_footprint_base_height(
    terrain: &crate::terrain::TerrainWorld,
    origin_tile: IVec2,
    size_tiles: IVec2,
) -> f32 {
    let tile_size = terrain.config.tile_size;
    let origin_x = origin_tile.x as f32 * tile_size;
    let origin_z = origin_tile.y as f32 * tile_size;

    let mut max_h = f32::NEG_INFINITY;
    for dz in 0..=size_tiles.y {
        for dx in 0..=size_tiles.x {
            let wx = origin_x + dx as f32 * tile_size;
            let wz = origin_z + dz as f32 * tile_size;
            let h = terrain.sample_height_at(wx, wz);
            if h > max_h {
                max_h = h;
            }
        }
    }

    if max_h.is_finite() { max_h } else { 0.0 }
}
