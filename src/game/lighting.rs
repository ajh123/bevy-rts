use bevy::prelude::*;

pub fn setup_sun_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 20_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.7, 0.0)),
    ));
}
