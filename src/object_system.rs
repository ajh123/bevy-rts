use glam::{IVec2, Mat4, Vec3};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ObjectHandle {
    index: u32,
    generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ObjectTypeId(pub(crate) u16);

#[derive(Clone, Debug)]
pub(crate) struct ObjectTypeSpec {
    pub(crate) name: String,
    /// Path relative to the Bevy asset root (the `assets/` folder).
    pub(crate) gltf: String,
    pub(crate) footprint_tiles: IVec2,
    pub(crate) gltf_bounds: Option<GltfBounds>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GltfBounds {
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl GltfBounds {
    pub(crate) fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub(crate) fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
}

/// OpenTTD-like "spec table" for object *types*.
///
/// Instances store only an `ObjectTypeId`, and tile data stores only an object index.
/// This keeps tile->object lookup fast and makes types data-driven.
#[derive(Default)]
pub(crate) struct ObjectTypeRegistry {
    specs: Vec<Option<ObjectTypeSpec>>,
    free_list: Vec<u16>,
}

impl ObjectTypeRegistry {
    pub(crate) fn register(&mut self, spec: ObjectTypeSpec) -> ObjectTypeId {
        if let Some(id) = self.free_list.pop() {
            self.specs[id as usize] = Some(spec);
            return ObjectTypeId(id);
        }

        let id = self.specs.len() as u16;
        self.specs.push(Some(spec));
        ObjectTypeId(id)
    }

    pub(crate) fn get(&self, id: ObjectTypeId) -> Option<&ObjectTypeSpec> {
        self.specs.get(id.0 as usize)?.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectInstance {
    pub(crate) type_id: ObjectTypeId,
    pub(crate) origin_tile: IVec2,
    pub(crate) size_tiles: IVec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaceError {
    InvalidFootprint,
    Occupied,
}

#[derive(Clone, Copy, Default)]
struct TileObjectSlot {
    // 0 means empty. Otherwise stores (object_index + 1).
    object_index_plus1: u32,
    #[allow(dead_code)]
    local_x: u8,
    #[allow(dead_code)]
    local_z: u8,
    flags: u16,
}

impl TileObjectSlot {
    const FLAG_ORIGIN: u16 = 1 << 0;

    fn is_empty(self) -> bool {
        self.object_index_plus1 == 0
    }

    fn object_index(self) -> Option<u32> {
        if self.object_index_plus1 == 0 {
            None
        } else {
            Some(self.object_index_plus1 - 1)
        }
    }

    fn is_origin(self) -> bool {
        (self.flags & Self::FLAG_ORIGIN) != 0
    }
}

struct ObjectSlot {
    generation: u32,
    instance: Option<ObjectInstance>,
}

struct ObjectChunk {
    tiles: Vec<TileObjectSlot>,
    dirty: bool,
}

impl ObjectChunk {
    fn new(chunk_size: i32) -> Self {
        let len = (chunk_size.max(1) as usize).pow(2);
        Self {
            tiles: vec![TileObjectSlot::default(); len],
            dirty: true,
        }
    }
}

/// OpenTTD-like object storage:
/// - O(1) tile -> object lookup via a per-tile packed reference
/// - stable-ish object handles via an indexed table + generation
/// - multi-tile objects occupy a footprint; each occupied tile points back to the same object
pub(crate) struct ObjectWorld {
    chunk_size: i32,
    chunks: HashMap<IVec2, ObjectChunk>,

    objects: Vec<ObjectSlot>,
    free_list: Vec<u32>,
}

impl ObjectWorld {
    pub(crate) fn new(chunk_size: i32) -> Self {
        Self {
            chunk_size: chunk_size.max(1),
            chunks: HashMap::new(),
            objects: Vec::new(),
            free_list: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn chunk_size(&self) -> i32 {
        self.chunk_size
    }

    pub(crate) fn object_at_tile(&self, tile: IVec2) -> Option<ObjectHandle> {
        let (chunk_coord, local) = tile_to_chunk_local(tile, self.chunk_size);
        let chunk = self.chunks.get(&chunk_coord)?;
        let slot = chunk.tiles[self.local_index(local)];
        let index = slot.object_index()?;
        let obj = self.objects.get(index as usize)?;
        let instance_exists = obj.instance.is_some();
        if !instance_exists {
            return None;
        }
        Some(ObjectHandle {
            index,
            generation: obj.generation,
        })
    }

    pub(crate) fn get(&self, handle: ObjectHandle) -> Option<&ObjectInstance> {
        let slot = self.objects.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.instance.as_ref()
    }

    pub(crate) fn try_place(
        &mut self,
        types: &ObjectTypeRegistry,
        type_id: ObjectTypeId,
        origin_tile: IVec2,
    ) -> Result<ObjectHandle, PlaceError> {
        let Some(spec) = types.get(type_id) else {
            return Err(PlaceError::InvalidFootprint);
        };

        let size_tiles = spec.footprint_tiles;
        if size_tiles.x <= 0 || size_tiles.y <= 0 || size_tiles.x > 255 || size_tiles.y > 255 {
            return Err(PlaceError::InvalidFootprint);
        }

        // First pass: ensure footprint is clear (does NOT allocate chunks).
        for dz in 0..size_tiles.y {
            for dx in 0..size_tiles.x {
                let t = origin_tile + IVec2::new(dx, dz);
                if self.tile_occupied(t) {
                    return Err(PlaceError::Occupied);
                }
            }
        }

        let handle = self.alloc(ObjectInstance {
            type_id,
            origin_tile,
            size_tiles,
        });

        // Second pass: write tile references (allocates chunks as needed).
        for dz in 0..size_tiles.y {
            for dx in 0..size_tiles.x {
                let t = origin_tile + IVec2::new(dx, dz);
                let (chunk_coord, local) = tile_to_chunk_local(t, self.chunk_size);
                let chunk = self
                    .chunks
                    .entry(chunk_coord)
                    .or_insert_with(|| ObjectChunk::new(self.chunk_size));

                let mut flags = 0u16;
                if dx == 0 && dz == 0 {
                    flags |= TileObjectSlot::FLAG_ORIGIN;
                }

                let idx = (local.y as usize) * (self.chunk_size as usize) + (local.x as usize);
                chunk.tiles[idx] = TileObjectSlot {
                    object_index_plus1: handle.index + 1,
                    local_x: dx as u8,
                    local_z: dz as u8,
                    flags,
                };
                chunk.dirty = true;
            }
        }

        Ok(handle)
    }

    pub(crate) fn remove_at_tile(&mut self, tile: IVec2) -> Option<ObjectHandle> {
        let handle = self.object_at_tile(tile)?;
        self.remove(handle)
    }

    pub(crate) fn remove(&mut self, handle: ObjectHandle) -> Option<ObjectHandle> {
        let Some(slot) = self.objects.get_mut(handle.index as usize) else {
            return None;
        };
        if slot.generation != handle.generation {
            return None;
        }
        let Some(instance) = slot.instance.take() else {
            return None;
        };

        // Clear footprint tiles.
        for dz in 0..instance.size_tiles.y {
            for dx in 0..instance.size_tiles.x {
                let t = instance.origin_tile + IVec2::new(dx, dz);
                let (chunk_coord, local) = tile_to_chunk_local(t, self.chunk_size);
                if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
                    let idx = (local.y as usize) * (self.chunk_size as usize) + (local.x as usize);
                    // Only clear if it still points to this object index.
                    if chunk.tiles[idx].object_index() == Some(handle.index) {
                        chunk.tiles[idx] = TileObjectSlot::default();
                        chunk.dirty = true;
                    }
                }
            }
        }

        // Free the handle (generation bump to invalidate stale references).
        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.free_list.push(handle.index);

        Some(handle)
    }

    pub(crate) fn chunk_is_dirty(&self, chunk_coord: IVec2) -> bool {
        self.chunks.get(&chunk_coord).map(|c| c.dirty).unwrap_or(false)
    }

    pub(crate) fn mark_chunk_clean(&mut self, chunk_coord: IVec2) {
        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            chunk.dirty = false;
        }
    }

    pub(crate) fn iter_origin_objects_in_chunk(
        &self,
        chunk_coord: IVec2,
    ) -> impl Iterator<Item = ObjectHandle> + '_ {
        let Some(chunk) = self.chunks.get(&chunk_coord) else {
            return OriginIter::Empty(std::iter::empty());
        };

        let iter = chunk.tiles.iter().filter_map(move |slot| {
            if slot.is_empty() || !slot.is_origin() {
                return None;
            }
            let index = slot.object_index()?;
            let obj = self.objects.get(index as usize)?;
            if obj.instance.is_none() {
                return None;
            }
            Some(ObjectHandle {
                index,
                generation: obj.generation,
            })
        });

        OriginIter::Some(iter)
    }

    fn tile_occupied(&self, tile: IVec2) -> bool {
        let (chunk_coord, local) = tile_to_chunk_local(tile, self.chunk_size);
        let Some(chunk) = self.chunks.get(&chunk_coord) else {
            return false;
        };
        !chunk.tiles[self.local_index(local)].is_empty()
    }

    fn alloc(&mut self, instance: ObjectInstance) -> ObjectHandle {
        if let Some(index) = self.free_list.pop() {
            let slot = &mut self.objects[index as usize];
            let generation = slot.generation.max(1);
            slot.instance = Some(instance);
            return ObjectHandle { index, generation };
        }

        let index = self.objects.len() as u32;
        self.objects.push(ObjectSlot {
            generation: 1,
            instance: Some(instance),
        });
        ObjectHandle {
            index,
            generation: 1,
        }
    }

    fn local_index(&self, local: IVec2) -> usize {
        (local.y as usize) * (self.chunk_size as usize) + (local.x as usize)
    }
}

enum OriginIter<I> {
    Some(I),
    Empty(std::iter::Empty<ObjectHandle>),
}

impl<I> Iterator for OriginIter<I>
where
    I: Iterator<Item = ObjectHandle>,
{
    type Item = ObjectHandle;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            OriginIter::Some(it) => it.next(),
            OriginIter::Empty(it) => it.next(),
        }
    }
}

fn tile_to_chunk_local(tile: IVec2, chunk_size: i32) -> (IVec2, IVec2) {
    let cs = chunk_size.max(1);
    let cx = tile.x.div_euclid(cs);
    let cz = tile.y.div_euclid(cs);
    let lx = tile.x.rem_euclid(cs);
    let lz = tile.y.rem_euclid(cs);
    (IVec2::new(cx, cz), IVec2::new(lx, lz))
}

// Bevy plumbing
use bevy::prelude::*;
use serde::Deserialize;
use crate::TerrainConfigRes;
use crate::selection::TileDoubleClicked;

#[derive(Resource)]
pub(crate) struct ObjectWorldRes(pub(crate) ObjectWorld);

#[derive(Resource)]
pub(crate) struct ObjectTypesRes {
    pub(crate) registry: ObjectTypeRegistry,
    pub(crate) test_building: ObjectTypeId,
}

pub(crate) fn setup_object_world(mut commands: Commands, config: Res<TerrainConfigRes>) {
    commands.insert_resource(ObjectWorldRes(ObjectWorld::new(config.0.chunk_size)));
}

pub(crate) fn setup_object_types(mut commands: Commands) {
    let mut registry = ObjectTypeRegistry::default();
    let mut loaded_ids = Vec::new();

    for def in load_object_type_defs_from_dir("assets/objects")
        .expect("failed to load object type definitions from assets/objects")
    {
        let bounds = try_compute_gltf_bounds_in_parent_space(&def.gltf).ok();
        let id = registry.register(ObjectTypeSpec {
            name: def.name,
            gltf: def.gltf,
            footprint_tiles: IVec2::new(def.footprint_tiles.0, def.footprint_tiles.1),
            gltf_bounds: bounds,
        });
        loaded_ids.push((id, registry.get(id).map(|s| s.name.clone()).unwrap_or_default()));
    }

    // Keep existing demo behavior: double-click toggles one specific object type.
    // Prefer "Small House" if present, otherwise fall back to the first loaded.
    let test_building = loaded_ids
        .iter()
        .find(|(_, name)| name == "Small House")
        .map(|(id, _)| *id)
        .or_else(|| loaded_ids.first().map(|(id, _)| *id))
        .unwrap_or_else(|| {
            // If no files exist, keep behavior deterministic.
            registry.register(ObjectTypeSpec {
                name: "MissingObjectDefs".to_string(),
                gltf: "".to_string(),
                footprint_tiles: IVec2::new(1, 1),
                gltf_bounds: None,
            })
        });

    commands.insert_resource(ObjectTypesRes {
        registry,
        test_building,
    });
}

/// Minimal demo behavior: double-click toggles a 2x2 "building" at that tile.
pub(crate) fn toggle_test_object_on_double_click(
    mut ev: MessageReader<TileDoubleClicked>,
    mut objects: ResMut<ObjectWorldRes>,
    types: Res<ObjectTypesRes>,
) {
    for e in ev.read() {
        if objects.0.object_at_tile(e.coord).is_some() {
            let _ = objects.0.remove_at_tile(e.coord);
        } else {
            let _ = objects
                .0
                .try_place(&types.registry, types.test_building, e.coord);
        }
    }
}

#[derive(Debug, Deserialize)]
struct ObjectTypeDefFile {
    name: String,
    gltf: String,
    footprint_tiles: (i32, i32),
}

fn load_object_type_defs_from_dir(
    dir: impl AsRef<std::path::Path>,
) -> Result<Vec<ObjectTypeDefFile>, String> {
    let dir = dir.as_ref();
    let mut defs = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read object defs dir '{}': {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read object defs dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ron") {
            continue;
        }

        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read object def '{}': {e}", path.display()))?;
        let def: ObjectTypeDefFile = ron::from_str(&text)
            .map_err(|e| format!("failed to parse object def '{}': {e}", path.display()))?;

        if def.name.trim().is_empty() {
            return Err(format!("object def '{}' has empty name", path.display()));
        }
        if def.gltf.trim().is_empty() {
            return Err(format!("object def '{}' has empty gltf path", path.display()));
        }
        if def.footprint_tiles.0 <= 0 || def.footprint_tiles.1 <= 0 {
            return Err(format!(
                "object def '{}' has invalid footprint_tiles {:?}",
                path.display(),
                def.footprint_tiles
            ));
        }

        defs.push(def);
    }

    Ok(defs)
}

fn try_compute_gltf_bounds_in_parent_space(asset_path: &str) -> Result<GltfBounds, String> {
    // Only supports JSON .gltf for now.
    if !asset_path.to_ascii_lowercase().ends_with(".gltf") {
        return Err("only .gltf is supported for bounds computation".to_string());
    }

    // Convert Bevy asset path (relative to assets/) into a filesystem path.
    let fs_path = std::path::Path::new("assets").join(asset_path);
    let text = std::fs::read_to_string(&fs_path)
        .map_err(|e| format!("failed to read gltf '{}': {e}", fs_path.display()))?;

    let doc: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse gltf json '{}': {e}", fs_path.display()))?;

    let meshes = doc
        .get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "gltf missing 'meshes'".to_string())?;
    let accessors = doc
        .get("accessors")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "gltf missing 'accessors'".to_string())?;

    // Find accessor indices used as POSITION for primitives.
    let mut position_accessor_indices: Vec<usize> = Vec::new();
    for mesh in meshes {
        let primitives = match mesh.get("primitives").and_then(|v| v.as_array()) {
            Some(p) => p,
            None => continue,
        };
        for prim in primitives {
            let attrs = match prim.get("attributes").and_then(|v| v.as_object()) {
                Some(a) => a,
                None => continue,
            };
            let Some(pos_idx) = attrs.get("POSITION").and_then(|v| v.as_u64()) else {
                continue;
            };
            position_accessor_indices.push(pos_idx as usize);
        }
    }
    if position_accessor_indices.is_empty() {
        return Err("gltf has no POSITION accessors".to_string());
    }

    // Merge AABB across all POSITION accessors.
    let mut local_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut local_max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

    for idx in position_accessor_indices {
        let Some(acc) = accessors.get(idx) else {
            continue;
        };
        let min = acc.get("min").and_then(|v| v.as_array());
        let max = acc.get("max").and_then(|v| v.as_array());
        let (Some(min), Some(max)) = (min, max) else {
            continue;
        };

        let read3 = |arr: &Vec<serde_json::Value>| -> Option<Vec3> {
            Some(Vec3::new(
                arr.get(0)?.as_f64()? as f32,
                arr.get(1)?.as_f64()? as f32,
                arr.get(2)?.as_f64()? as f32,
            ))
        };

        let Some(min_v) = read3(min) else { continue; };
        let Some(max_v) = read3(max) else { continue; };

        local_min = local_min.min(min_v);
        local_max = local_max.max(max_v);
    }

    if !local_min.is_finite() || !local_max.is_finite() {
        return Err("failed to compute finite bounds from accessors".to_string());
    }

    // Apply default scene's root node matrix (if present) to get bounds in parent space.
    let root_transform = try_read_default_scene_root_matrix(&doc).unwrap_or(Mat4::IDENTITY);
    let (min_p, max_p) = transform_aabb(root_transform, local_min, local_max);

    Ok(GltfBounds { min: min_p, max: max_p })
}

fn try_read_default_scene_root_matrix(doc: &serde_json::Value) -> Option<Mat4> {
    let scene_index = doc.get("scene").and_then(|v| v.as_u64())? as usize;
    let scenes = doc.get("scenes").and_then(|v| v.as_array())?;
    let scene = scenes.get(scene_index)?;
    let root_nodes = scene.get("nodes").and_then(|v| v.as_array())?;
    // Handle the common case: exactly one root node with a matrix.
    let root_idx = root_nodes.get(0)?.as_u64()? as usize;
    let nodes = doc.get("nodes").and_then(|v| v.as_array())?;
    let root = nodes.get(root_idx)?;

    if let Some(m) = root.get("matrix").and_then(|v| v.as_array()) {
        if m.len() == 16 {
            let mut f = [0.0f32; 16];
            for (i, v) in m.iter().enumerate() {
                f[i] = v.as_f64()? as f32;
            }
            // glTF matrices are column-major.
            return Some(Mat4::from_cols_array(&f));
        }
    }

    None
}

fn transform_aabb(m: Mat4, min: Vec3, max: Vec3) -> (Vec3, Vec3) {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
    ];

    let mut out_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut out_max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

    for c in corners {
        let p = m.transform_point3(c);
        out_min = out_min.min(p);
        out_max = out_max.max(p);
    }

    (out_min, out_max)
}
