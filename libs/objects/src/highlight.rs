use bevy::prelude::*;

/// Spawns a hologram preview entity if it doesn't exist, or updates it if it does.
///
/// Returns the preview entity.
pub fn update_hologram(
    commands: &mut Commands,
    preview_entity: Option<Entity>,
    preview_scene_child: Option<Entity>,
    scene_handle: Handle<Scene>,
    transform: Transform,
    scene_offset_local: Vec3,
) -> (Entity, Entity) {
    let root = if let Some(e) = preview_entity {
        commands.entity(e).insert(transform);
        e
    } else {
        commands.spawn((transform, Visibility::default())).id()
    };

    let child = if let Some(c) = preview_scene_child {
        commands
            .entity(c)
            .insert(Transform::from_translation(scene_offset_local));
        c
    } else {
        let c = commands
            .spawn((
                SceneRoot(scene_handle),
                Transform::from_translation(scene_offset_local),
                Visibility::default(),
            ))
            .id();
        commands.entity(root).add_child(c);
        c
    };

    (root, child)
}

/// Recursively applies a material override to all MeshMaterial3d<StandardMaterial> components
/// in the hierarchy, useful for "hologram" or "blueprint" effects.
pub fn apply_hologram_material_recursive(
    children: &Query<&Children>,
    materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    entity: Entity,
    material: &Handle<StandardMaterial>,
    depth: usize,
) {
    if let Ok(mut mat) = materials.get_mut(entity) {
        if mat.0 != *material {
            mat.0 = material.clone();
        }
    }

    if depth > 100 {
        warn_once!("apply_hologram_material_recursive hit recursion limit");
        return;
    }

    if let Ok(kids) = children.get(entity) {
        for child in kids.iter() {
            apply_hologram_material_recursive(children, materials, child, material, depth + 1);
        }
    }
}

/// Despawns an entity and all its descendants.
pub fn despawn_recursive(commands: &mut Commands, children: &Query<&Children>, entity: Entity) {
    if let Ok(kids) = children.get(entity) {
        for child in kids.iter() {
            despawn_recursive(commands, children, child);
        }
    }
    commands.entity(entity).despawn();
}
