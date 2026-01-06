use bevy::prelude::*;
use bevy::mesh::PrimitiveTopology;
use bevy::asset::RenderAssetUsages;
use glam::{IVec2, Vec2 as GVec2};
use crate::camera::TopDownCamera;
use crate::terrain_renderer::TerrainWorldRes;


#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct SelectedTile {
    pub(crate) coord: Option<IVec2>,
}

#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct DoubleClickState {
    pending: Option<(IVec2, f32)>,
    last_tile: Option<IVec2>,
    last_click_time_secs: f32,
}

#[derive(Component)]
pub(crate) struct SelectionHighlight;

#[derive(Component, Clone, Copy)]
pub(crate) struct HighlightForTile(IVec2);

/// Handle mouse clicks to select tiles.
pub(crate) fn handle_mouse_selection(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<TopDownCamera>>,
    terrain: Res<TerrainWorldRes>,
    mut selected_tile: ResMut<SelectedTile>,
    time: Res<Time>,
    mut double_click: ResMut<DoubleClickState>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = match windows.single() {
        Ok(w) => w,
        Err(_) => return,
    };

    let (camera, camera_transform) = match camera_q.single() {
        Ok(c) => c,
        Err(_) => return,
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Proper ray from camera through cursor (works for perspective + orthographic).
    let ray = match camera.viewport_to_world(camera_transform, cursor_pos) {
        Ok(r) => r,
        Err(_) => return,
    };

    // Intersect the camera ray with the procedural terrain surface (heightfield),
    // so selection stays accurate and the highlight can follow the mesh.
    let Some(hit_point) = raycast_to_heightfield(&terrain.0, ray) else {
        return;
    };

    let tile_coord = terrain.0.world_to_tile_coord(hit_point.x, hit_point.z);

    selected_tile.coord = Some(tile_coord);

    // Simple double-click detection: two clicks on the same tile within a small time window.
    const DOUBLE_CLICK_WINDOW_SECS: f32 = 0.75;
    let now = time.elapsed_secs();
    let is_double_click = match double_click.pending {
        Some((last_tile, last_time))
            if last_tile == tile_coord && (now - last_time) <= DOUBLE_CLICK_WINDOW_SECS =>
        {
            // Consume the pending click so we don't treat subsequent rapid clicks
            // as repeated double-clicks.
            double_click.pending = None;
            true
        }
        _ => {
            double_click.pending = Some((tile_coord, now));
            false
        }
    };

    // Keep these fields for the expected behavior/state tracking.
    double_click.last_tile = Some(tile_coord);
    double_click.last_click_time_secs = now;

    if is_double_click {
        println!("Double-clicked on tile {:?}", tile_coord);
    }
}

/// Render the selection highlight square.
pub(crate) fn render_selection_highlight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_tile: Res<SelectedTile>,
    terrain: Res<TerrainWorldRes>,
    mut query: Query<(Entity, &mut Transform, Option<&HighlightForTile>), With<SelectionHighlight>>,
) {
    match selected_tile.coord {
        Some(coord) => {
            let tile_center = terrain.0.tile_center(coord);

            // We build the highlight mesh so it follows the terrain heights along the outline.
            // Since the geometry depends on which tile is selected, respawn if selection changes.
            if let Ok((entity, mut transform, existing_tile)) = query.single_mut() {
                let needs_respawn = existing_tile.map(|t| t.0 != coord).unwrap_or(true);
                if needs_respawn {
                    commands.entity(entity).despawn();
                } else {
                    // Keep XZ centered; mesh vertices carry their own Y heights.
                    transform.translation = Vec3::new(tile_center.x, 0.0, tile_center.y);
                    return;
                }
            }

            let mesh = create_conforming_outline_mesh(&terrain.0, coord);
            let mesh_handle = meshes.add(mesh);
            let material = materials.add(StandardMaterial {
                base_color: Color::BLACK,
                unlit: true,
                cull_mode: None,
                ..default()
            });

            commands.spawn((
                SelectionHighlight,
                HighlightForTile(coord),
                Mesh3d(mesh_handle),
                MeshMaterial3d(material),
                Transform::from_translation(Vec3::new(tile_center.x, 0.0, tile_center.y)),
            ));
        }
        None => {
            // Remove highlight if nothing is selected
            if let Ok((entity, _, _)) = query.single_mut() {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn raycast_to_heightfield(terrain: &crate::terrain::TerrainWorld, ray: Ray3d) -> Option<Vec3> {
    // Only handle rays pointing downwards.
    if ray.direction.y >= -1e-4 {
        return None;
    }

    // We step along the ray until we go below the heightfield, then refine with binary search.
    // This avoids needing physics/collision meshes.
    let max_depth_y = -200.0;
    let t_max = ((ray.origin.y - max_depth_y) / (-ray.direction.y)).clamp(0.0, 10_000.0);
    if t_max <= 0.0 {
        return None;
    }

    let step_y = (terrain.config.tile_size * 0.5).clamp(0.25, 2.0);
    let step_t = (step_y / (-ray.direction.y)).clamp(0.01, 5.0);

    let mut prev_t = 0.0;
    let mut prev_p = ray.origin;
    let mut prev_h = terrain.sample_height_at(prev_p.x, prev_p.z);

    let mut t = step_t;
    while t <= t_max {
        let p = ray.origin + ray.direction * t;
        let h = terrain.sample_height_at(p.x, p.z);

        if p.y <= h {
            // Bracketed: prev is above, current is below.
            let mut lo = prev_t;
            let mut hi = t;

            for _ in 0..12 {
                let mid = 0.5 * (lo + hi);
                let mp = ray.origin + ray.direction * mid;
                let mh = terrain.sample_height_at(mp.x, mp.z);
                if mp.y <= mh {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }

            let hit_t = hi;
            let hit_p = ray.origin + ray.direction * hit_t;
            let hit_h = terrain.sample_height_at(hit_p.x, hit_p.z);
            return Some(Vec3::new(hit_p.x, hit_h, hit_p.z));
        }

        prev_t = t;
        prev_p = p;
        prev_h = h;
        t += step_t;
    }

    // If we started below the terrain (rare), treat it as a hit at origin projection.
    if prev_p.y <= prev_h {
        return Some(Vec3::new(prev_p.x, prev_h, prev_p.z));
    }

    None
}

/// Build an outline mesh that conforms to the terrain surface around a tile.
///
/// The returned mesh is centered on the tile center in XZ, and its Y values are absolute (world) heights.
fn create_conforming_outline_mesh(
    terrain: &crate::terrain::TerrainWorld,
    tile_coord: IVec2,
) -> Mesh {
    let tile_size = terrain.config.tile_size;
    let half = tile_size * 0.5;
    let thickness = (tile_size * 0.08).clamp(0.08, 0.25);
    let lift = 0.25;

    let tile_center = terrain.tile_center(tile_coord);

    // Sample multiple points per edge so the outline follows height changes.
    let segments_per_edge: usize = 8;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Helper to push a ribbon along an edge.
    let mut add_edge = |start: GVec2, end: GVec2, inward_normal: GVec2| {
        let base_index = positions.len() as u32;

        for i in 0..=segments_per_edge {
            let u = i as f32 / segments_per_edge as f32;
            let p_outer = start.lerp(end, u);
            let p_inner = p_outer + inward_normal * thickness;

            let world_outer = tile_center + p_outer;
            let world_inner = tile_center + p_inner;

            let h_outer = terrain.sample_height_at(world_outer.x, world_outer.y) + lift;
            let h_inner = terrain.sample_height_at(world_inner.x, world_inner.y) + lift;

            positions.push([p_outer.x, h_outer, p_outer.y]);
            positions.push([p_inner.x, h_inner, p_inner.y]);
        }

        // Build quads between consecutive samples.
        for i in 0..segments_per_edge {
            let o0 = base_index + (i * 2) as u32;
            let i0 = o0 + 1;
            let o1 = o0 + 2;
            let i1 = o0 + 3;

            indices.extend_from_slice(&[o0, i0, o1]);
            indices.extend_from_slice(&[i0, i1, o1]);
        }
    };

    // Local (tile-centered) coordinates are (x, z) stored in Vec2(x, z).
    // Bottom edge: z = -half, inward normal points +z
    add_edge(
        GVec2::new(-half, -half),
        GVec2::new(half, -half),
        GVec2::Y,
    );
    // Right edge: x = +half, inward normal points -x
    add_edge(
        GVec2::new(half, -half),
        GVec2::new(half, half),
        -GVec2::X,
    );
    // Top edge: z = +half, inward normal points -z
    add_edge(
        GVec2::new(half, half),
        GVec2::new(-half, half),
        -GVec2::Y,
    );
    // Left edge: x = -half, inward normal points +x
    add_edge(
        GVec2::new(-half, half),
        GVec2::new(-half, -half),
        GVec2::X,
    );

    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let uvs = vec![[0.0, 0.0]; positions.len()];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}
