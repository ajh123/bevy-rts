use bevy::prelude::*;

use crate::game::camera::UiInputCaptureRes;
use crate::game::ui::toolbar::{ToolbarActionText, ToolbarRegistry, ToolbarState, ToolbarTool};
use crate::game::world::objects::system::{HoveredObjectRes, ObjectTypesRes, ObjectWorldRes};
use crate::game::world::terrain::types::TerrainWorldRes;

pub struct DestructionModePlugin;

impl Plugin for DestructionModePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_destruction_toolbar)
            .add_systems(
                Update,
                (
                    draw_hover_highlight,
                    handle_destruction_click,
                    draw_destruction_ui,
                ),
            );
    }
}

fn setup_destruction_toolbar(mut registry: ResMut<ToolbarRegistry>) {
    registry.tools.push(ToolbarTool {
        id: "destroy".to_string(),
        label: "Destroy".to_string(),
        order: 10,
        key: Some(KeyCode::Digit2),
    });
}

fn draw_destruction_ui(toolbar: Res<ToolbarState>, mut action_text: ResMut<ToolbarActionText>) {
    if toolbar.active_tool.as_deref() != Some("destroy") {
        return;
    }

    action_text.0 = "Mode: Destroy\nLMB: Remove hovered object".to_string();
}

fn draw_hover_highlight(
    mut gizmos: Gizmos,
    hovered: Res<HoveredObjectRes>,
    objects: Res<ObjectWorldRes>,
    types: Res<ObjectTypesRes>,
    toolbar: Res<ToolbarState>,
    terrain: Res<TerrainWorldRes>,
) {
    if toolbar.active_tool.as_deref() != Some("destroy") {
        return;
    }

    let Some(handle) = hovered.0 else {
        return;
    };

    let Some(instance) = objects.0.get(handle) else {
        return;
    };

    let Some(spec) = types.registry.get(instance.type_id) else {
        return;
    };

    let base_h = terrain
        .0
        .sample_height_at(instance.position_world.x, instance.position_world.z);

    gizmos.circle(
        Isometry3d::new(
            Vec3::new(
                instance.position_world.x,
                base_h + 0.1,
                instance.position_world.z,
            ),
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ),
        spec.hover_radius,
        Color::WHITE,
    );
}

fn handle_destruction_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    toolbar: Res<ToolbarState>,
    mut objects: ResMut<ObjectWorldRes>,
    hovered: Res<HoveredObjectRes>,
    ui_capture: Res<UiInputCaptureRes>,
) {
    if ui_capture.pointer {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if toolbar.active_tool.as_deref() == Some("destroy") {
        if let Some(h) = hovered.0 {
            let _ = objects.0.remove(h);
        }
    }
}
