use crate::{
    engine::object::{object::ObjectId, object_collection::ObjectCollection},
    helper::{axis::CartesianAxis, more_errors::CollectionError},
    user_interface::camera::Camera,
};
use glam::{DVec2, Vec4, Vec4Swizzles};

#[derive(Default, Debug, Clone, Copy)]
pub struct GizmoVisibility {
    pub linear: bool,
    // rotate
    // linear_plane
    // scale
}

impl GizmoVisibility {
    pub fn any_visible(&self) -> bool {
        return self.linear; // || self.rotate || ...
    }
    pub fn hide_all(&mut self) {
        self.linear = false;
    }
    pub fn show_all(&mut self) {
        self.linear = true;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GizmoElement {
    Linear(CartesianAxis),
    // Rotate
    // LinearPlane
    // Scale
}

impl Default for GizmoElement {
    fn default() -> Self {
        Self::Linear(Default::default())
    }
}

impl GizmoElement {
    pub fn process_dragged(
        &self,
        cursor_delta: DVec2,
        selected_object_id: ObjectId,
        object_collection: &mut ObjectCollection,
        camera: &Camera,
    ) -> Result<(), CollectionError> {
        match *self {
            Self::Linear(axis) => process_translate(
                axis,
                cursor_delta,
                selected_object_id,
                object_collection,
                camera,
            ),
        }
    }
}

fn process_translate(
    axis: CartesianAxis,
    cursor_delta: DVec2,
    selected_object_id: ObjectId,
    object_collection: &mut ObjectCollection,
    camera: &Camera,
) -> Result<(), CollectionError> {
    let object_center = object_collection.get_object(selected_object_id)?.center;
    let center_projected = camera.view_matrix() * Vec4::from((object_center, 1.));
    let depth = -center_projected.z; // distance from camera to object center

    let axis_projected =
        camera.projection_matrix() * camera.view_matrix() * Vec4::from((axis.as_vec3(), 1.));
    let translation_abs = cursor_delta.dot(axis_projected.xy().as_dvec2()) as f32 * depth / 2000.;
    let translation_vec = axis.as_vec3() * translation_abs;

    object_collection.translate_object(selected_object_id, translation_vec)
}
