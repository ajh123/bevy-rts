pub mod assets;
pub mod highlight;
pub mod spatial;
pub mod system;
pub mod types;

pub use types::*;

use bevy::prelude::*;

pub struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
	fn build(&self, app: &mut App) {
		app.init_asset::<assets::ObjectTypeDefAsset>()
			.init_asset_loader::<assets::ObjectTypeDefAssetLoader>()
			.init_asset::<assets::BinaryAsset>()
			.init_asset_loader::<assets::BinaryAssetLoader>()
			.init_resource::<system::CursorHit>()
			.init_resource::<spatial::SpatialHashGrid>()
			.add_systems(Startup, (system::setup_object_types, system::setup_object_hovered))
			.add_systems(
				Update,
				(
					system::finish_object_types_load,
					spatial::spatial_index_added,
					spatial::spatial_index_changed,
					spatial::spatial_index_removed,
					system::update_hovered_object,
				),
			);
	}
}
