use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use parrot::Perlin;
use std::collections::{HashMap, HashSet, VecDeque};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.60, 0.80, 0.95)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 300.0,
            affects_lightmapped_meshes: false,
        })
        .insert_resource(TerrainConfig::default())
        .insert_resource(TopDownCameraSettings::default())
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_viewer, setup_terrain_resources))
        .add_systems(
            Update,
            (
                top_down_camera_input,
                update_top_down_camera.after(top_down_camera_input),
                stream_chunks,
            ),
        )
        .run();
}

#[derive(Resource, Clone)]
struct TerrainConfig {
    seed: u64,
    chunk_size: i32,
    tile_size: f32,
    view_distance_chunks: i32,
    chunk_spawn_budget_per_frame: usize,
    noise_base_frequency: f64,
    noise_octaves: u32,
    noise_persistence: f64,
    height_scale: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            chunk_size: 32,
            tile_size: 2.0,
            view_distance_chunks: 8,
            chunk_spawn_budget_per_frame: 32,
            noise_base_frequency: 0.02,
            noise_octaves: 4,
            noise_persistence: 0.5,
            height_scale: 8.0,
        }
    }
}

#[derive(Resource, Clone)]
struct TerrainNoise {
    perlin: Perlin,
}

#[derive(Resource, Clone)]
struct TerrainAtlas {
    material: Handle<StandardMaterial>,
    tile_count: f32,
}

#[derive(Resource, Default)]
struct LoadedChunks {
    entities: HashMap<IVec2, Entity>,
}

#[derive(Resource, Default)]
struct ChunkStreamingState {
    last_viewer_chunk: Option<IVec2>,
    desired: HashSet<IVec2>,
    pending_spawn: VecDeque<IVec2>,
    pending_despawn: VecDeque<(IVec2, Entity)>,
}

#[derive(Component)]
struct Viewer;

#[derive(Component)]
struct TopDownCamera;

#[derive(Resource, Clone)]
struct TopDownCameraSettings {
    yaw: f32,
    pitch: f32,
    distance: f32,
    min_distance: f32,
    max_distance: f32,
    pan_speed: f32,
    pan_speed_fast: f32,
    rotate_speed: f32,
    zoom_speed: f32,
    mouse_pan_sensitivity: f32,
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

#[derive(Component)]
struct Chunk {
    #[allow(dead_code)]
    coord: IVec2,
}

fn setup_viewer(mut commands: Commands) {
    commands.spawn((Viewer, Transform::from_xyz(0.0, 0.0, 0.0)));

    commands.spawn((
        TopDownCamera,
        Camera3d::default(),
        Transform::default(),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -0.8,
            0.7,
            0.0,
        )),
    ));
}

fn setup_terrain_resources(
    mut commands: Commands,
    config: Res<TerrainConfig>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TerrainNoise {
        perlin: Perlin::new(config.seed),
    });
    commands.insert_resource(LoadedChunks::default());
    commands.insert_resource(ChunkStreamingState::default());

    // Tiny in-memory atlas: [water, sand, grass, rock, snow]
    // Each "tile" in the heightmap selects one of these texels via UVs.
    let atlas_colors = [
        Color::srgb(0.10, 0.25, 0.80),
        Color::srgb(0.85, 0.80, 0.55),
        Color::srgb(0.15, 0.60, 0.20),
        Color::srgb(0.45, 0.45, 0.50),
        Color::srgb(0.95, 0.95, 0.98),
    ];

    let atlas_tex = images.add(make_atlas_1x_n_image(&atlas_colors));
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(atlas_tex),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.insert_resource(TerrainAtlas {
        material,
        tile_count: atlas_colors.len() as f32,
    });
}

fn make_atlas_1x_n_image(colors: &[Color]) -> Image {
    let mut data = Vec::with_capacity(colors.len() * 4);
    for c in colors {
        let [r, g, b, a] = c.to_srgba().to_u8_array();
        data.extend_from_slice(&[r, g, b, a]);
    }

    let mut image = Image::new(
        Extent3d {
            width: colors.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::nearest();
    image
}

fn top_down_camera_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut settings: ResMut<TopDownCameraSettings>,
    mut q_focus: Query<&mut Transform, With<Viewer>>,
) {
    let mut focus = match q_focus.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    // Rotate around focus
    if keys.pressed(KeyCode::KeyQ) {
        settings.yaw += settings.rotate_speed * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyE) {
        settings.yaw -= settings.rotate_speed * time.delta_secs();
    }

    // Zoom
    let mut scroll: f32 = 0.0;
    for ev in mouse_wheel.read() {
        scroll += ev.y;
    }
    if scroll.abs() > 0.0 {
        // Exponential-ish feel, similar to city builder cameras.
        let factor = (1.0 - scroll * settings.zoom_speed).clamp(0.2, 5.0);
        settings.distance = (settings.distance * factor).clamp(settings.min_distance, settings.max_distance);
    }

    // Pan (keyboard) on XZ plane, relative to camera yaw.
    let mut input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        input.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        input.x += 1.0;
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

fn update_top_down_camera(
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

fn stream_chunks(
    mut commands: Commands,
    config: Res<TerrainConfig>,
    noise: Res<TerrainNoise>,
    atlas: Res<TerrainAtlas>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut loaded: ResMut<LoadedChunks>,
    mut streaming: ResMut<ChunkStreamingState>,
    q_viewer: Query<&Transform, With<Viewer>>,
) {
    let viewer_pos = match q_viewer.single() {
        Ok(v) => v.translation,
        Err(_) => return,
    };
    let chunk_world_size = config.chunk_size as f32 * config.tile_size;
    let viewer_chunk = IVec2::new(
        (viewer_pos.x / chunk_world_size).floor() as i32,
        (viewer_pos.z / chunk_world_size).floor() as i32,
    );

    // Recompute streaming targets only when entering a new chunk.
    if streaming.last_viewer_chunk != Some(viewer_chunk) {
        streaming.last_viewer_chunk = Some(viewer_chunk);

        streaming.desired.clear();
        for dz in -config.view_distance_chunks..=config.view_distance_chunks {
            for dx in -config.view_distance_chunks..=config.view_distance_chunks {
                streaming.desired.insert(viewer_chunk + IVec2::new(dx, dz));
            }
        }

        streaming.pending_spawn.clear();
        let desired_coords: Vec<IVec2> = streaming.desired.iter().copied().collect();
        for coord in desired_coords {
            if !loaded.entities.contains_key(&coord) {
                streaming.pending_spawn.push_back(coord);
            }
        }

        streaming.pending_despawn.clear();
        for (coord, entity) in loaded.entities.iter() {
            if !streaming.desired.contains(coord) {
                streaming.pending_despawn.push_back((*coord, *entity));
            }
        }
    }

    // Incremental despawn/spawn to avoid massive spikes at large view distances.
    let mut budget = config.chunk_spawn_budget_per_frame;
    while budget > 0 {
        let Some((coord, entity)) = streaming.pending_despawn.pop_front() else {
            break;
        };
        if loaded.entities.remove(&coord).is_some() {
            commands.entity(entity).despawn();
        }
        budget -= 1;
    }

    let mut budget = config.chunk_spawn_budget_per_frame;
    while budget > 0 {
        let Some(coord) = streaming.pending_spawn.pop_front() else {
            break;
        };
        if loaded.entities.contains_key(&coord) {
            budget -= 1;
            continue;
        }
        let chunk_entity = spawn_chunk(&mut commands, &mut meshes, &config, &noise, &atlas, coord);
        loaded.entities.insert(coord, chunk_entity);
        budget -= 1;
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    config: &TerrainConfig,
    noise: &TerrainNoise,
    atlas: &TerrainAtlas,
    coord: IVec2,
) -> Entity {
    let chunk_world_size = config.chunk_size as f32 * config.tile_size;
    let chunk_origin = Vec3::new(
        coord.x as f32 * chunk_world_size,
        0.0,
        coord.y as f32 * chunk_world_size,
    );

    let mesh = build_chunk_mesh(config, &noise.perlin, coord, atlas.tile_count);
    let mesh_handle = meshes.add(mesh);

    commands
        .spawn((
            Chunk { coord },
            Mesh3d(mesh_handle),
            MeshMaterial3d(atlas.material.clone()),
            Transform::from_translation(chunk_origin),
        ))
        .id()
}

fn build_chunk_mesh(
    config: &TerrainConfig,
    perlin: &Perlin,
    coord: IVec2,
    atlas_tile_count: f32,
) -> Mesh {
    let chunk_world_size = config.chunk_size as f32 * config.tile_size;
    let chunk_origin_x = coord.x as f32 * chunk_world_size;
    let chunk_origin_z = coord.y as f32 * chunk_world_size;

    let n = config.chunk_size.max(1) as usize;
    let stride = n + 1;
    let tile_size = config.tile_size;

    // Pre-sample heights once per grid vertex (huge perf win vs per-tile sampling).
    let mut heights: Vec<f32> = vec![0.0; stride * stride];
    for gz in 0..=n {
        for gx in 0..=n {
            let wx = chunk_origin_x + gx as f32 * tile_size;
            let wz = chunk_origin_z + gz as f32 * tile_size;
            heights[gz * stride + gx] = sample_height(config, perlin, wx, wz);
        }
    }

    // Derive smooth normals from the height grid (no extra noise samples).
    let mut normals_grid: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; stride * stride];
    for gz in 0..=n {
        for gx in 0..=n {
            let gx_l = gx.saturating_sub(1);
            let gx_r = (gx + 1).min(n);
            let gz_d = gz.saturating_sub(1);
            let gz_u = (gz + 1).min(n);

            let h_l = heights[gz * stride + gx_l];
            let h_r = heights[gz * stride + gx_r];
            let h_d = heights[gz_d * stride + gx];
            let h_u = heights[gz_u * stride + gx];

            let dx = ((gx_r as i32 - gx_l as i32).max(1) as f32) * tile_size;
            let dz = ((gz_u as i32 - gz_d as i32).max(1) as f32) * tile_size;

            let dhdx = (h_r - h_l) / dx;
            let dhdz = (h_u - h_d) / dz;

            let normal = Vec3::new(-dhdx, 1.0, -dhdz).normalize_or_zero();
            normals_grid[gz * stride + gx] = [normal.x, normal.y, normal.z];
        }
    }

    let tile_count = (n * n) as usize;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(tile_count * 4);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(tile_count * 4);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(tile_count * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(tile_count * 6);

    for z in 0..n {
        for x in 0..n {
            let x0 = x as f32 * tile_size;
            let z0 = z as f32 * tile_size;
            let x1 = x0 + tile_size;
            let z1 = z0 + tile_size;

            let h00 = heights[z * stride + x];
            let h10 = heights[z * stride + (x + 1)];
            let h01 = heights[(z + 1) * stride + x];
            let h11 = heights[(z + 1) * stride + (x + 1)];

            let n00 = normals_grid[z * stride + x];
            let n10 = normals_grid[z * stride + (x + 1)];
            let n01 = normals_grid[(z + 1) * stride + x];
            let n11 = normals_grid[(z + 1) * stride + (x + 1)];

            let avg_h = (h00 + h10 + h01 + h11) * 0.25;
            let tile_index = pick_tile_index(avg_h);
            let uv_u = (tile_index as f32 + 0.5) / atlas_tile_count;
            let uv = [uv_u, 0.5];

            let v0 = Vec3::new(x0, h00, z0);
            let v1 = Vec3::new(x1, h10, z0);
            let v2 = Vec3::new(x0, h01, z1);
            let v3 = Vec3::new(x1, h11, z1);

            let base = positions.len() as u32;
            positions.extend_from_slice(&[
                [v0.x, v0.y, v0.z],
                [v1.x, v1.y, v1.z],
                [v2.x, v2.y, v2.z],
                [v3.x, v3.y, v3.z],
            ]);
            normals.extend_from_slice(&[
                n00,
                n10,
                n01,
                n11,
            ]);
            uvs.extend_from_slice(&[uv, uv, uv, uv]);

            // Winding chosen so the "top" faces upward (CCW when viewed from above).
            indices.extend_from_slice(&[
                base,
                base + 2,
                base + 1,
                base + 1,
                base + 2,
                base + 3,
            ]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn sample_height(config: &TerrainConfig, perlin: &Perlin, world_x: f32, world_z: f32) -> f32 {
    let mut amplitude = 1.0f64;
    let mut frequency = config.noise_base_frequency;
    let mut sum = 0.0f64;
    let mut norm = 0.0f64;

    for _ in 0..config.noise_octaves {
        let n = perlin.noise2d(world_x as f64 * frequency, world_z as f64 * frequency);
        sum += n * amplitude;
        norm += amplitude;
        amplitude *= config.noise_persistence;
        frequency *= 2.0;
    }

    let value = if norm > 0.0 { sum / norm } else { 0.0 };
    (value as f32) * config.height_scale
}

fn pick_tile_index(height: f32) -> u32 {
    // 0..=4 maps to the atlas order: [water, sand, grass, rock, snow]
    if height < -3.0 {
        0
    } else if height < -1.0 {
        1
    } else if height < 3.0 {
        2
    } else if height < 6.0 {
        3
    } else {
        4
    }
}