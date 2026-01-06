use crate::camera::Viewer;
use crate::terrain::{TerrainAction, TerrainWorld};
use crate::TerrainConfigRes;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use glam::{IVec2, Vec2};
use std::collections::HashMap;

#[derive(Component)]
pub(crate) struct Chunk {
    #[allow(dead_code)]
    coord: IVec2,
}

#[derive(Resource)]
pub(crate) struct TerrainWorldRes(pub(crate) TerrainWorld);

#[derive(Resource)]
pub(crate) struct TerrainAtlas {
    material: Handle<StandardMaterial>,
    tile_count: f32,
}

#[derive(Resource, Default)]
pub(crate) struct LoadedChunkEntities {
    entities: HashMap<IVec2, Entity>,
}

pub fn setup_terrain_renderer(
    mut commands: Commands,
    config: Res<TerrainConfigRes>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TerrainWorldRes(TerrainWorld::new(config.0.clone())));
    commands.insert_resource(LoadedChunkEntities::default());

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

pub fn stream_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    atlas: Res<TerrainAtlas>,
    mut terrain: ResMut<TerrainWorldRes>,
    mut loaded: ResMut<LoadedChunkEntities>,
    q_viewer: Query<&Transform, With<Viewer>>,
) {
    let viewer_pos = match q_viewer.single() {
        Ok(v) => v.translation,
        Err(_) => return,
    };

    terrain
        .0
        .set_viewer_world_xz(Vec2::new(viewer_pos.x, viewer_pos.z));
    let actions = terrain.0.tick();

    for action in actions {
        match action {
            TerrainAction::DespawnChunk(coord) => {
                if let Some(entity) = loaded.entities.remove(&coord) {
                    commands.entity(entity).despawn();
                }
            }
            TerrainAction::SpawnChunk(coord) => {
                if loaded.entities.contains_key(&coord) {
                    continue;
                }

                let chunk_entity = spawn_chunk(
                    &mut commands,
                    &mut meshes,
                    &terrain.0,
                    &atlas,
                    coord,
                );
                loaded.entities.insert(coord, chunk_entity);
            }
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    terrain: &TerrainWorld,
    atlas: &TerrainAtlas,
    coord: IVec2,
) -> Entity {
    let origin = terrain.chunk_origin_world(coord);
    let mesh_data = terrain.build_chunk_mesh_data(coord, atlas.tile_count);
    let mesh = mesh_from_chunk_mesh_data(mesh_data);
    let mesh_handle = meshes.add(mesh);

    commands
        .spawn((
            Chunk { coord },
            Mesh3d(mesh_handle),
            MeshMaterial3d(atlas.material.clone()),
            Transform::from_translation(Vec3::new(origin.x, origin.y, origin.z)),
        ))
        .id()
}

fn mesh_from_chunk_mesh_data(data: crate::terrain::ChunkMeshData) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs);
    mesh.insert_indices(Indices::U32(data.indices));
    mesh
}
