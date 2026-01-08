use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;

#[derive(Asset, TypePath, Debug, Clone)]
pub struct ObjectTypeDefAsset {
    pub name: String,
    pub gltf: String,
    pub render_scale: Vec3,
    pub hover_radius: f32,
    pub scene_offset_local: Vec3,
}

#[derive(Asset, TypePath, Debug, Clone)]
pub struct BinaryAsset(pub Vec<u8>);

#[derive(Default)]
pub struct BinaryAssetLoader;

impl AssetLoader for BinaryAssetLoader {
    type Asset = BinaryAsset;
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
        Ok(BinaryAsset(bytes))
    }

    fn extensions(&self) -> &[&str] {
        &["bin"]
    }
}

#[derive(Default)]
pub struct ObjectTypeDefAssetLoader;

impl AssetLoader for ObjectTypeDefAssetLoader {
    type Asset = ObjectTypeDefAsset;
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
            .map_err(|e| format!("object def was not valid utf-8: {e}"))?;

        let def: ObjectTypeDefFile = ron::from_str(text)
            .map_err(|e| format!("failed to parse object def ron: {e}"))?;

        if def.name.trim().is_empty() {
            return Err("object def has empty name".to_string());
        }
        if def.gltf.trim().is_empty() {
            return Err("object def has empty gltf path".to_string());
        }
        if !def.hover_radius.is_finite() || def.hover_radius <= 0.0 {
            return Err(format!("object def '{}' has invalid hover_radius={}", def.name, def.hover_radius));
        }

        Ok(ObjectTypeDefAsset {
            name: def.name,
            gltf: def.gltf,
            render_scale: Vec3::new(def.scale.0, def.scale.1, def.scale.2),
            hover_radius: def.hover_radius,
            scene_offset_local: Vec3::new(
                def.scene_offset_local.0,
                def.scene_offset_local.1,
                def.scene_offset_local.2,
            ),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

#[derive(Debug, Deserialize)]
struct ObjectTypeDefFile {
    name: String,
    gltf: String,
    #[serde(default = "default_object_scale")]
    scale: Scale3,
    hover_radius: f32,
    scene_offset_local: Vec3File,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct Scale3(pub(crate) f32, pub(crate) f32, pub(crate) f32);

#[derive(Clone, Copy, Debug, Deserialize)]
struct Vec3File(pub(crate) f32, pub(crate) f32, pub(crate) f32);

fn default_object_scale() -> Scale3 {
    Scale3(1.0, 1.0, 1.0)
}
