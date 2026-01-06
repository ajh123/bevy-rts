use bevy::prelude::*;
use glam::IVec2;

use crate::object_system::{ObjectTypesRes, ObjectWorldRes};
use crate::terrain_renderer::{LoadedChunkEntities, TerrainWorldRes};

#[derive(Resource)]
pub(crate) struct ObjectRenderAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.70, 0.15, 0.10),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.insert_resource(ObjectRenderAssets { mesh, material });
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
    assets: Res<ObjectRenderAssets>,
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

        // Spawn render parts for each object origin tile in this chunk.
        let mut to_spawn = Vec::new();
        for handle in objects.0.iter_origin_objects_in_chunk(root.coord) {
            let Some(instance) = objects.0.get(handle) else {
                continue;
            };

            let Some(spec) = types.registry.get(instance.type_id) else {
                continue;
            };

            // Compute a conservative base height using the max height under the footprint.
            // This avoids objects clipping into sloped terrain.
            let base_h = compute_footprint_base_height(&terrain.0, instance.origin_tile, instance.size_tiles);

            let origin_corner_x = instance.origin_tile.x as f32 * tile_size;
            let origin_corner_z = instance.origin_tile.y as f32 * tile_size;
            let object_center_x = origin_corner_x + (instance.size_tiles.x as f32) * tile_size * 0.5;
            let object_center_z = origin_corner_z + (instance.size_tiles.y as f32) * tile_size * 0.5;

            let parts = (spec.vtable.build_render_parts)(instance);
            for part in parts {
                let part_center_x = object_center_x + part.center_offset_tiles.x * tile_size;
                let part_center_z = object_center_z + part.center_offset_tiles.y * tile_size;

                let sx = part.size_tiles.x * tile_size;
                let sz = part.size_tiles.y * tile_size;
                let sy = part.height;
                let y = base_h + sy * 0.5 + part.y_offset;

                // IMPORTANT: these are spawned as CHILDREN of the chunk root.
                // Child transforms are local to the root, so convert world->chunk-local.
                let local_pos = Vec3::new(part_center_x - chunk_origin.x, y, part_center_z - chunk_origin.z);
                to_spawn.push((local_pos, Vec3::new(sx, sy, sz)));
            }
        }

        commands.entity(root_entity).with_children(|parent| {
            for (pos, scale) in to_spawn.drain(..) {
                parent.spawn((
                    Mesh3d(assets.mesh.clone()),
                    MeshMaterial3d(assets.material.clone()),
                    Transform::from_translation(pos).with_scale(scale),
                    Visibility::default(),
                    // Per-instance tint would require a custom material; keep one material for now.
                    // Color is currently unused but kept for future expansion.
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
