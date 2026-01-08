use bevy::prelude::*;

use objects::highlight;
use objects::system::{HoveredObjectRes, ObjectKind, ObjectTypesRes};
use terrain::types::TerrainWorldRes;
use ui::{ToolbarActionText, ToolbarRegistry, ToolbarState, ToolbarTool, UiInputCaptureRes};

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
    types: Res<ObjectTypesRes>,
    toolbar: Res<ToolbarState>,
    terrain: Res<TerrainWorldRes>,
    q_objects: Query<(&Transform, &ObjectKind)>,
) {
    if toolbar.active_tool.as_deref() != Some("destroy") {
        return;
    }

    let Some(entity) = hovered.0 else {
        return;
    };

    let Ok((transform, kind)) = q_objects.get(entity) else {
        return;
    };

    let Some(spec) = types.registry.get(kind.0) else {
        return;
    };

    let base_h = terrain
        .0
        .sample_height_at(transform.translation.x, transform.translation.z);

    gizmos.circle(
        Isometry3d::new(
            Vec3::new(
                transform.translation.x,
                base_h + 0.1,
                transform.translation.z,
            ),
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ),
        spec.hover_radius,
        Color::WHITE,
    );
}

fn handle_destruction_click(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    toolbar: Res<ToolbarState>,
    hovered: Res<HoveredObjectRes>,
    ui_capture: Res<UiInputCaptureRes>,
    children: Query<&Children>,
) {
    if ui_capture.pointer {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if toolbar.active_tool.as_deref() == Some("destroy") {
        if let Some(entity) = hovered.0 {
            highlight::despawn_recursive(&mut commands, &children, entity);
        }
    }
}
