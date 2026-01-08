use bevy::prelude::*;
use glam::{Vec2, Vec3 as GVec3};

use objects::system::CursorHitRes;
use terrain::types::{TerrainViewerWorldXzRes, TerrainWorldRes};
use ui::UiInputCaptureRes;

use crate::game::camera::TopDownCamera;
use crate::game::camera::Viewer;
use bevy_egui::EguiContexts;

pub(crate) fn update_ui_input_capture(
    mut contexts: EguiContexts,
    mut capture: ResMut<UiInputCaptureRes>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => {
            capture.pointer = false;
            capture.keyboard = false;
            return;
        }
    };

    // Pointer capture: egui wants pointer OR cursor is over egui.
    // (The "over area" check avoids world clicks when hovering UI widgets.)
    capture.pointer = ctx.wants_pointer_input() || ctx.is_pointer_over_area();

    // Keyboard capture: egui wants keyboard (this is usually true when a text field is active).
    capture.keyboard = ctx.wants_keyboard_input();
}

/// Update the current mouse cursor hit point against the procedural terrain heightfield.
pub(crate) fn update_cursor_hit(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<TopDownCamera>>,
    terrain: Res<TerrainWorldRes>,
    mut hit: ResMut<CursorHitRes>,
    ui_capture: Res<UiInputCaptureRes>,
) {
    if ui_capture.pointer {
        hit.world = None;
        return;
    }

    let window = match windows.single() {
        Ok(w) => w,
        Err(_) => {
            hit.world = None;
            return;
        }
    };

    let (camera, camera_transform) = match camera_q.single() {
        Ok(c) => c,
        Err(_) => {
            hit.world = None;
            return;
        }
    };

    let Some(cursor_pos) = window.cursor_position() else {
        hit.world = None;
        return;
    };

    let ray = match camera.viewport_to_world(camera_transform, cursor_pos) {
        Ok(r) => r,
        Err(_) => {
            hit.world = None;
            return;
        }
    };

    let Some(hit_point) = crate::game::physics::raycast::raycast_to_heightfield(&terrain.0, ray)
    else {
        hit.world = None;
        return;
    };

    hit.world = Some(GVec3::new(hit_point.x, hit_point.y, hit_point.z));
}

pub(crate) fn update_terrain_viewer_world_xz(
    q_viewer: Query<&Transform, With<Viewer>>,
    mut viewer_xz: ResMut<TerrainViewerWorldXzRes>,
) {
    let Ok(t) = q_viewer.single() else {
        return;
    };
    viewer_xz.0 = Vec2::new(t.translation.x, t.translation.z);
}
