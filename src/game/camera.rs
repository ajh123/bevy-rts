use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use ui::UiInputCapture;

use super::{StartupSet, UpdateSet};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TopDownCameraSettings::default())
            .add_systems(Startup, setup_viewer.in_set(StartupSet::Camera))
            .add_systems(Update, top_down_camera_input.in_set(UpdateSet::CameraInput))
            .add_systems(Update, update_top_down_camera.in_set(UpdateSet::CameraUpdate));
    }
}

#[derive(Component)]
pub struct Viewer;

#[derive(Component)]
pub struct TopDownCamera;

#[derive(Resource, Clone)]
pub struct TopDownCameraSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub pan_speed: f32,
    pub pan_speed_fast: f32,
    pub rotate_speed: f32,
    pub zoom_speed: f32,
    pub mouse_pan_sensitivity: f32,
}

impl Default for TopDownCameraSettings {
    fn default() -> Self {
        Self {
            yaw: 0.8,
            pitch: 1.05,
            distance: 80.0,
            min_distance: 10.0,
            max_distance: 400.0,
            pan_speed: 60.0,
            pan_speed_fast: 180.0,
            rotate_speed: 1.8,
            zoom_speed: 0.12,
            mouse_pan_sensitivity: 0.12,
        }
    }
}

pub fn setup_viewer(mut commands: Commands) {
    commands.spawn((Viewer, Transform::from_xyz(0.0, 0.0, 0.0)));

    commands.spawn((TopDownCamera, Camera3d::default(), Transform::default()));
}

pub fn top_down_camera_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut settings: ResMut<TopDownCameraSettings>,
    mut q_focus: Query<&mut Transform, With<Viewer>>,
    ui_capture: Res<UiInputCapture>,
) {
    let mut focus = match q_focus.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    // Keyboard input: ignore while egui is actively consuming keyboard input (e.g. text field).
    if !ui_capture.keyboard {
        // Rotate around focus
        if keys.pressed(KeyCode::KeyQ) {
            settings.yaw += settings.rotate_speed * time.delta_secs();
        }
        if keys.pressed(KeyCode::KeyE) {
            settings.yaw -= settings.rotate_speed * time.delta_secs();
        }
    }

    // Pointer input: ignore while cursor is over / interacting with egui.
    if !ui_capture.pointer {
        // Zoom
        let mut scroll: f32 = 0.0;
        for ev in mouse_wheel.read() {
            scroll += ev.y;
        }
        if scroll.abs() > 0.0 {
            // Exponential-ish feel, similar to city builder cameras.
            let factor = (1.0 - scroll * settings.zoom_speed).clamp(0.2, 5.0);
            settings.distance =
                (settings.distance * factor).clamp(settings.min_distance, settings.max_distance);
        }
    }

    // Pan (keyboard) on XZ plane, relative to camera yaw.
    let mut input = Vec2::ZERO;
    if !ui_capture.keyboard {
        if keys.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            input.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.x -= 1.0;
        }
    }

    let yaw_rot = Quat::from_rotation_y(settings.yaw);
    let right = yaw_rot * Vec3::X;
    let forward = yaw_rot * Vec3::Z;

    if input.length_squared() > 0.0 {
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            settings.pan_speed_fast
        } else {
            settings.pan_speed
        };

        let delta = (right * input.x + forward * input.y) * speed * time.delta_secs();
        focus.translation += Vec3::new(delta.x, 0.0, delta.z);
    }

    // Pan (mouse drag): middle mouse button drags the world under the cursor.
    if !ui_capture.pointer {
        if mouse_buttons.pressed(MouseButton::Middle) {
            let mut drag = Vec2::ZERO;
            for ev in mouse_motion.read() {
                drag += ev.delta;
            }
            if drag.length_squared() > 0.0 {
                let scale = settings.mouse_pan_sensitivity * (settings.distance / 80.0);
                // Screen-space: +x right, +y up. Dragging right should move focus left.
                let delta = (-right * drag.x + forward * drag.y) * scale;
                focus.translation += Vec3::new(delta.x, 0.0, delta.z);
            }
        }
    }
}

pub fn update_top_down_camera(
    settings: Res<TopDownCameraSettings>,
    q_focus: Query<&Transform, (With<Viewer>, Without<TopDownCamera>)>,
    mut q_cam: Query<&mut Transform, (With<TopDownCamera>, Without<Viewer>)>,
) {
    let focus = match q_focus.single() {
        Ok(v) => v.translation,
        Err(_) => return,
    };
    let mut cam = match q_cam.single_mut() {
        Ok(c) => c,
        Err(_) => return,
    };

    let rot = Quat::from_euler(EulerRot::YXZ, settings.yaw, settings.pitch, 0.0);
    let offset = rot * Vec3::new(0.0, 0.0, -settings.distance);
    cam.translation = focus + offset;
    cam.look_at(focus, Vec3::Y);
}
