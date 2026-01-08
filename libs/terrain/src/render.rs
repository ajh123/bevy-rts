use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use glam::{IVec2, Vec3};

use crate::assets::TileTypesAsset;
use crate::types::TileTypes;
use crate::types::{LoadedChunkEntities, TerrainAtlas, TerrainConfig, TerrainViewerWorldXz};
use crate::world::{ChunkMeshData, TerrainAction, TerrainWorld};

#[derive(Component)]
pub struct Chunk;

#[derive(Resource, Clone)]
pub struct TileTypesHandle(pub Handle<TileTypesAsset>);

pub fn setup_terrain_renderer(
    mut commands: Commands,
    config: Res<TerrainConfig>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(TerrainWorld::new(config.clone()));
    commands.insert_resource(LoadedChunkEntities::default());

    let handle: Handle<TileTypesAsset> = asset_server.load("tiles.ron");
    commands.insert_resource(TileTypesHandle(handle));
}

pub fn finish_tile_types_load(
    mut commands: Commands,
    handle: Option<Res<TileTypesHandle>>,
    assets: Res<Assets<TileTypesAsset>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(handle) = handle else {
        return;
    };

    let Some(asset) = assets.get(&handle.0) else {
        return;
    };
    let tile_types = asset.0.clone();

    commands.remove_resource::<TileTypesHandle>();

    let atlas_colors: Vec<Color> = tile_types
        .tiles
        .iter()
        .map(|t| {
            let (r, g, b) = t.color_srgb;
            Color::srgb(r, g, b)
        })
        .collect();

    commands.insert_resource(tile_types);

    let atlas_tex = images.add(make_atlas_1x_n_image(&atlas_colors));
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(atlas_tex),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.insert_resource(TerrainAtlas { material });
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
    atlas: Option<Res<TerrainAtlas>>,
    tiles: Option<Res<TileTypes>>,
    mut terrain: ResMut<TerrainWorld>,
    mut loaded: ResMut<LoadedChunkEntities>,
    viewer: Res<TerrainViewerWorldXz>,
) {
    let (Some(atlas), Some(tiles)) = (atlas, tiles) else {
        return;
    };

    terrain.set_viewer_world_xz(viewer.0);
    let actions = terrain.tick();

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

                let chunk_entity =
                    spawn_chunk(&mut commands, &mut meshes, &terrain, &atlas, &tiles, coord);
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
    tiles: &TileTypes,
    coord: IVec2,
) -> Entity {
    let origin = terrain.chunk_origin_world(coord);
    let mesh_data = terrain.build_chunk_mesh_data(coord, tiles);
    let mesh = mesh_from_chunk_mesh_data(mesh_data);
    let mesh_handle = meshes.add(mesh);

    commands
        .spawn((
            Chunk,
            Mesh3d(mesh_handle),
            MeshMaterial3d(atlas.material.clone()),
            Transform::from_translation(Vec3::new(origin.x, origin.y, origin.z)),
        ))
        .id()
}

fn mesh_from_chunk_mesh_data(data: ChunkMeshData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs);
    mesh.insert_indices(Indices::U32(data.indices));
    mesh
}
