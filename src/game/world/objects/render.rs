use bevy::prelude::*;
use bevy::scene::SceneRoot;
use std::collections::HashMap;

use super::system::{ObjectTypesRes, ObjectWorldRes};
use super::types::ObjectHandle;
use crate::game::utils::highlight;
use crate::game::world::terrain::types::TerrainWorldRes;

#[derive(Resource, Default)]
pub struct ObjectEntityMap(pub HashMap<ObjectHandle, Entity>);

pub fn sync_objects(
    mut commands: Commands,
    mut objects: ResMut<ObjectWorldRes>,
    mut entity_map: ResMut<ObjectEntityMap>,
    types: Res<ObjectTypesRes>,
    terrain: Res<TerrainWorldRes>,
    asset_server: Res<AssetServer>,
    children_query: Query<&Children>,
) {
    if objects.0.added_handles.is_empty() && objects.0.removed_handles.is_empty() {
        return;
    }

    // Process removed objects
    for handle in objects.0.removed_handles.iter() {
        if let Some(entity) = entity_map.0.remove(handle) {
            highlight::despawn_recursive(&mut commands, &children_query, entity);
        }
    }

    // Process added objects
    for handle in objects.0.added_handles.iter() {
        let Some(instance) = objects.0.get(*handle) else {
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

        let position = Vec3::new(instance.position_world.x, base_h, instance.position_world.z);

        let scene_handle =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));

        let rot = Quat::from_rotation_y(instance.yaw);
        let rotated_offset = rot
            * Vec3::new(
                spec.render_offset.x,
                spec.render_offset.y,
                spec.render_offset.z,
            );

        let entity = commands
            .spawn((
                SceneRoot(scene_handle),
                Transform::from_translation(position + rotated_offset)
                    .with_rotation(rot)
                    .with_scale(spec.render_scale),
                Visibility::default(),
            ))
            .id();

        entity_map.0.insert(*handle, entity);
    }

    objects.0.clear_events();
}
