use std::collections::{HashMap, HashSet};
use glam::{IVec2, Vec3};
use super::types::{
    ObjectHandle, ObjectTypeId, ObjectTypeRegistry,
    FreeformObjectInstance
};
use crate::game::physics::collision;

struct FreeformObjectSlot {
    generation: u32,
    instance: Option<FreeformObjectInstance>,
    chunk: IVec2,
}

pub struct FreeformObjectWorld {
    chunk_world_size: f32,
    objects: Vec<FreeformObjectSlot>,
    free_list: Vec<u32>,
    by_chunk: HashMap<IVec2, Vec<u32>>,
    dirty_chunks: HashSet<IVec2>,
}

impl FreeformObjectWorld {
    pub fn new(chunk_size: i32, tile_size: f32) -> Self {
        let chunk_world_size = (chunk_size.max(1) as f32) * tile_size.max(1e-3);
        Self {
            chunk_world_size,
            objects: Vec::new(),
            free_list: Vec::new(),
            by_chunk: HashMap::new(),
            dirty_chunks: HashSet::new(),
        }
    }

    pub fn chunk_is_dirty(&self, chunk_coord: IVec2) -> bool {
        self.dirty_chunks.contains(&chunk_coord)
    }

    pub fn mark_chunk_clean(&mut self, chunk_coord: IVec2) {
        self.dirty_chunks.remove(&chunk_coord);
    }

    pub fn get(&self, handle: ObjectHandle) -> Option<&FreeformObjectInstance> {
        let slot = self.objects.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.instance.as_ref()
    }

    pub fn iter_objects_in_chunk(
        &self,
        chunk_coord: IVec2,
    ) -> Box<dyn Iterator<Item = ObjectHandle> + '_> {
        let Some(indices) = self.by_chunk.get(&chunk_coord) else {
            return Box::new(std::iter::empty());
        };

        Box::new(indices.iter().copied().filter_map(|index| {
            let slot = self.objects.get(index as usize)?;
            if slot.instance.is_none() {
                return None;
            }
            Some(ObjectHandle {
                index,
                generation: slot.generation,
            })
        }))
    }

    pub fn place(&mut self, type_id: ObjectTypeId, position_world: Vec3, yaw: f32) -> ObjectHandle {
        let chunk = self.world_to_chunk_coord(position_world);
        let handle = self.alloc(FreeformObjectInstance {
            type_id,
            position_world,
            yaw,
        }, chunk);

        self.by_chunk.entry(chunk).or_default().push(handle.index);
        self.dirty_chunks.insert(chunk);
        handle
    }

    pub fn remove(&mut self, handle: ObjectHandle) -> Option<ObjectHandle> {
        let Some(slot) = self.objects.get_mut(handle.index as usize) else {
            return None;
        };
        if slot.generation != handle.generation {
            return None;
        }
        let _instance = slot.instance.take()?;
        let chunk = slot.chunk;

        if let Some(v) = self.by_chunk.get_mut(&chunk) {
            v.retain(|idx| *idx != handle.index);
            if v.is_empty() {
                self.by_chunk.remove(&chunk);
            }
        }

        self.dirty_chunks.insert(chunk);

        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.free_list.push(handle.index);

        Some(handle)
    }

    pub fn pick_hovered(
        &self,
        types: &ObjectTypeRegistry,
        cursor_world: Vec3,
    ) -> Option<ObjectHandle> {
        let center_chunk = self.world_to_chunk_coord(cursor_world);

        let mut best: Option<(ObjectHandle, f32)> = None;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let c = center_chunk + IVec2::new(dx, dz);
                let Some(indices) = self.by_chunk.get(&c) else {
                    continue;
                };
                for idx in indices.iter().copied() {
                    let slot = self.objects.get(idx as usize)?;
                    let inst = match &slot.instance {
                        Some(i) => i,
                        None => continue,
                    };
                    let spec = match types.get(inst.type_id) {
                        Some(s) => s,
                        None => continue,
                    };

                    let r = collision::collision_radius(spec.gltf_bounds.as_ref(), spec.render_scale, spec.hover_radius);
                    
                    if !collision::point_in_circle(cursor_world, inst.position_world, r) {
                        continue;
                    }

                    // For best hit (closest to center), we still check d2
                    let dx = inst.position_world.x - cursor_world.x;
                    let dz = inst.position_world.z - cursor_world.z;
                    let d2 = dx * dx + dz * dz;

                    if best.map(|(_, b)| d2 < b).unwrap_or(true) {
                        best = Some((
                            ObjectHandle {
                                index: idx,
                                generation: slot.generation,
                            },
                            d2,
                        ));
                    }
                }
            }
        }

        best.map(|(h, _)| h)
    }

    pub fn can_place_non_overlapping(
        &self,
        types: &ObjectTypeRegistry,
        type_id: ObjectTypeId,
        position_world: Vec3,
    ) -> bool {
        let Some(new_spec) = types.get(type_id) else {
            return false;
        };

        let new_r = collision::collision_radius(new_spec.gltf_bounds.as_ref(), new_spec.render_scale, new_spec.hover_radius);
        let center_chunk = self.world_to_chunk_coord(position_world);

        // Query a small neighborhood of chunks. We keep this conservative and cheap.
        for dz in -1..=1 {
            for dx in -1..=1 {
                let c = center_chunk + IVec2::new(dx, dz);
                let Some(indices) = self.by_chunk.get(&c) else {
                    continue;
                };

                for idx in indices.iter().copied() {
                    let Some(slot) = self.objects.get(idx as usize) else {
                        continue;
                    };
                    let Some(inst) = &slot.instance else {
                        continue;
                    };
                    let Some(spec) = types.get(inst.type_id) else {
                        continue;
                    };

                    let other_r = collision::collision_radius(spec.gltf_bounds.as_ref(), spec.render_scale, spec.hover_radius);
                    
                    if collision::circles_overlap(position_world, new_r, inst.position_world, other_r) {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn alloc(&mut self, instance: FreeformObjectInstance, chunk: IVec2) -> ObjectHandle {
        if let Some(index) = self.free_list.pop() {
            let slot = &mut self.objects[index as usize];
            let generation = slot.generation.max(1);
            slot.instance = Some(instance);
            slot.chunk = chunk;
            return ObjectHandle { index, generation };
        }

        let index = self.objects.len() as u32;
        self.objects.push(FreeformObjectSlot {
            generation: 1,
            instance: Some(instance),
            chunk,
        });

        ObjectHandle {
            index,
            generation: 1,
        }
    }

    fn world_to_chunk_coord(&self, world: Vec3) -> IVec2 {
        let cs = self.chunk_world_size.max(1e-3);
        IVec2::new((world.x / cs).floor() as i32, (world.z / cs).floor() as i32)
    }
}
