pub mod assets;
pub mod render;
pub mod types;
pub mod world;

pub use types::*;
pub use world::*;

use bevy::prelude::*;

pub struct TerrainPlugin {
    pub config: types::TerrainConfig,
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .init_resource::<types::TerrainViewerWorldXz>()
            .init_asset::<assets::TileTypesAsset>()
            .init_asset_loader::<assets::TileTypesAssetLoader>()
            .add_systems(Startup, render::setup_terrain_renderer)
            .add_systems(
                Update,
                (render::finish_tile_types_load, render::stream_chunks),
            );
    }
}
