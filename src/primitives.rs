use crate::shaders::shader_interfaces::{primitve_codes, PrimitiveDataUnit, PRIMITIVE_LEN};
use glam::Vec3;

/// bruh
type PrimitiveDataSlice = [PrimitiveDataUnit; PRIMITIVE_LEN];

#[derive(Default, Debug, Clone, Copy)]
pub struct Sphere {
    pub radius: f32,
    pub center: Vec3,
}
impl Sphere {
    pub fn new(radius: f32, center: Vec3) -> Self {
        Self { radius, center }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Primitives {
    /// Encoded primitive data.
    data: Vec<PrimitiveDataUnit>,
    spheres: Vec<Sphere>,
}
// Public functions
impl Primitives {
    pub fn encoded_data(&self) -> &Vec<PrimitiveDataUnit> {
        &self.data
    }

    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.spheres.push(sphere);
        self.data.extend_from_slice(&Self::encode_sphere(sphere));
    }

    pub fn spheres(&self) -> &Vec<Sphere> {
        &self.spheres
    }

    pub fn update_sphere(&mut self, index: usize, new_sphere: Sphere) {
        if let Some(s_ref) = self.spheres.get_mut(index) {
            let encoded = Self::encode_sphere(new_sphere);
            let data_start = index * PRIMITIVE_LEN;
            let data_end = data_start + PRIMITIVE_LEN;
            self.data.splice(data_start..data_end, encoded);
            *s_ref = new_sphere;
        } else {
            todo!();
        }
    }
}
// Private functions
impl Primitives {
    fn encode_sphere(sphere: Sphere) -> PrimitiveDataSlice {
        [
            primitve_codes::SPHERE,
            sphere.radius.to_bits(),
            sphere.center.x.to_bits(),
            sphere.center.y.to_bits(),
            sphere.center.z.to_bits(),
            // padding
            primitve_codes::NULL,
            primitve_codes::NULL,
            primitve_codes::NULL,
        ]
    }
}
