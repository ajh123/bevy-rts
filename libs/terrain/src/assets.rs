use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;

use crate::types::{TileTypes, TileTypesFile};

#[derive(Asset, TypePath, Debug, Clone)]
pub struct TileTypesAsset(pub TileTypes);

#[derive(Default)]
pub struct TileTypesAssetLoader;

impl AssetLoader for TileTypesAssetLoader {
    type Asset = TileTypesAsset;
    type Settings = ();
    type Error = String;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| format!("failed to read asset bytes: {e}"))?;

        let text = std::str::from_utf8(&bytes)
            .map_err(|e| format!("tile types asset was not valid utf-8: {e}"))?;

        let parsed: TileTypesFile =
            ron::from_str(text).map_err(|e| format!("failed to parse tile types ron: {e}"))?;

        let tile_types = TileTypes {
            tiles: parsed.tiles,
        };
        tile_types.validate()?;

        Ok(TileTypesAsset(tile_types))
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}
