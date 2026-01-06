use glam::{IVec2, Vec2};
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
    #[allow(dead_code)]
    pub(crate) name: String,

    /// OpenTTD-style polymorphism: behavior is provided by per-type callbacks.
    pub(crate) vtable: ObjectTypeVTable,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ObjectTypeVTable {
    pub(crate) footprint_tiles: fn() -> IVec2,
    pub(crate) build_render_parts: fn(&ObjectInstance) -> Vec<ObjectRenderPart>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ObjectRenderPart {
    /// Center offset relative to the object center, in tiles (x,z).
    pub(crate) center_offset_tiles: Vec2,
    /// Part size in tiles (x,z).
    pub(crate) size_tiles: Vec2,
    /// Part height in world units.
    pub(crate) height: f32,
    /// Extra lift from the computed terrain base.
    pub(crate) y_offset: f32,
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

        let size_tiles = (spec.vtable.footprint_tiles)();
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
    let test_building = registry.register(ObjectTypeSpec {
        name: "Test Building".to_string(),
        vtable: ObjectTypeVTable {
            footprint_tiles: test_building_footprint_tiles,
            build_render_parts: test_building_render_parts,
        },
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

fn test_building_footprint_tiles() -> IVec2 {
    IVec2::new(2, 2)
}

fn test_building_render_parts(inst: &ObjectInstance) -> Vec<ObjectRenderPart> {
    vec![ObjectRenderPart {
        center_offset_tiles: Vec2::ZERO,
        size_tiles: Vec2::new(inst.size_tiles.x as f32, inst.size_tiles.y as f32),
        height: 1.5,
        y_offset: 0.0,
    }]
}
