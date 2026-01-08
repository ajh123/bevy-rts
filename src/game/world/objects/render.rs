use bevy::prelude::*;
use bevy::scene::SceneRoot;

use crate::game::utils::highlight;
use crate::game::world::terrain::types::TerrainWorldRes;

use super::system::{FreeformObjectWorldRes, ObjectTypesRes};

pub fn spawn_objects_for_chunks(
    mut commands: Commands,
    mut objects: ResMut<FreeformObjectWorldRes>,
    terrain: Res<TerrainWorldRes>,
    types: Res<ObjectTypesRes>,
    loaded_chunks: Query<(Entity, &crate::game::world::terrain::render::Chunk)>,
    children_query: Query<&Children>,
    asset_server: Res<AssetServer>,
) {
    for (root_entity, root) in loaded_chunks.iter() {
        if !objects.0.chunk_is_dirty(root.coord) {
            continue;
        }

        // Manual despawn_descendants
        if let Ok(kids) = children_query.get(root_entity) {
            for child in kids {
                // Use Bevy's built-in recursive despawn if available, otherwise manual
                // But commands.entity(e).despawn_recursive() requires DespawnRecursiveExt.
                // We can use the one from utils which I have to make public and use here.
                // Or just commands.entity(*child).despawn_recursive(); assuming preamble works?
                // Let's use our utils one.
                highlight::despawn_recursive(&mut commands, &children_query, *child);
            }
        }

        let chunk_size = terrain.0.config.chunk_size as f32;
        let tile_size = terrain.0.config.tile_size;
        let chunk_world_size = chunk_size * tile_size;
        let chunk_origin = Vec3::new(
            root.coord.x as f32 * chunk_world_size,
            0.0,
            root.coord.y as f32 * chunk_world_size,
        );

        // Pre-calculate what to spawn to avoid borrow checker issues with `objects`.
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

            let scene_handle =
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));
            let rot = Quat::from_rotation_y(instance.yaw);
            let rotated_offset = rot
                * Vec3::new(
                    spec.render_offset.x,
                    spec.render_offset.y,
                    spec.render_offset.z,
                );
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
