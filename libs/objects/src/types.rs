use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectTypeId(pub u16);

#[derive(Clone, Debug)]
pub struct ObjectTypeSpec {
    pub name: String,
    /// Path relative to the Bevy asset root (the `assets/` folder).
    pub gltf: String,
    pub render_scale: Vec3,
    pub hover_radius: f32,
    /// Local translation applied to the rendered scene child.
    ///
    /// This must be authored in the object definition file; it is not computed at runtime.
    pub scene_offset_local: Vec3,
}

#[derive(Default)]
pub struct ObjectTypeRegistry {
    specs: Vec<Option<ObjectTypeSpec>>,
    free_list: Vec<u16>,
}

impl ObjectTypeRegistry {
    pub fn register(&mut self, spec: ObjectTypeSpec) -> ObjectTypeId {
        if let Some(id) = self.free_list.pop() {
            self.specs[id as usize] = Some(spec);
            return ObjectTypeId(id);
        }

        let id = self.specs.len() as u16;
        self.specs.push(Some(spec));
        ObjectTypeId(id)
    }

    pub fn get(&self, id: ObjectTypeId) -> Option<&ObjectTypeSpec> {
        self.specs.get(id.0 as usize)?.as_ref()
    }
}
