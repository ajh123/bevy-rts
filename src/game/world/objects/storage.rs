use super::types::{ObjectHandle, ObjectInstance, ObjectTypeId, ObjectTypeRegistry};
use crate::game::physics::collision;
use glam::Vec3;
use std::collections::HashSet;

struct ObjectSlot {
    generation: u32,
    instance: Option<ObjectInstance>,
}

pub struct ObjectWorld {
    objects: Vec<ObjectSlot>,
    free_list: Vec<u32>,
    pub added_handles: HashSet<ObjectHandle>,
    pub removed_handles: HashSet<ObjectHandle>,
}

impl ObjectWorld {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            free_list: Vec::new(),
            added_handles: HashSet::new(),
            removed_handles: HashSet::new(),
        }
    }

    pub fn clear_events(&mut self) {
        self.added_handles.clear();
        self.removed_handles.clear();
    }

    pub fn get(&self, handle: ObjectHandle) -> Option<&ObjectInstance> {
        let slot = self.objects.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.instance.as_ref()
    }

    pub fn place(&mut self, type_id: ObjectTypeId, position_world: Vec3, yaw: f32) -> ObjectHandle {
        let handle = self.alloc(ObjectInstance {
            type_id,
            position_world,
            yaw,
        });

        self.added_handles.insert(handle);
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

        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.free_list.push(handle.index);

        // Remove from added if it was just added this frame
        if self.added_handles.contains(&handle) {
            self.added_handles.remove(&handle);
        } else {
            self.removed_handles.insert(handle);
        }

        Some(handle)
    }

    pub fn pick_hovered(
        &self,
        types: &ObjectTypeRegistry,
        cursor_world: Vec3,
    ) -> Option<ObjectHandle> {
        let mut best: Option<(ObjectHandle, f32)> = None;

        // Iterate all objects
        for (index, slot) in self.objects.iter().enumerate() {
            let Some(inst) = &slot.instance else {
                continue;
            };
            let Some(spec) = types.get(inst.type_id) else {
                continue;
            };

            let r = collision::collision_radius(
                spec.gltf_bounds.as_ref(),
                spec.render_scale,
                spec.hover_radius,
            );

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
                        index: index as u32,
                        generation: slot.generation,
                    },
                    d2,
                ));
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

        let new_r = collision::collision_radius(
            new_spec.gltf_bounds.as_ref(),
            new_spec.render_scale,
            new_spec.hover_radius,
        );

        // Iterate all objects
        for slot in self.objects.iter() {
            let Some(inst) = &slot.instance else {
                continue;
            };
            let Some(spec) = types.get(inst.type_id) else {
                continue;
            };

            let other_r = collision::collision_radius(
                spec.gltf_bounds.as_ref(),
                spec.render_scale,
                spec.hover_radius,
            );

            if collision::circles_overlap(position_world, new_r, inst.position_world, other_r) {
                return false;
            }
        }

        true
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
}
