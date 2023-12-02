use super::{gui::EditState, gui_state::DRAG_INC};
use crate::{
    config,
    engine::{
        object::primitive_op::PrimitiveOp,
        primitives::{
            cube::Cube, primitive_transform::PrimitiveTransform, sphere::Sphere,
            uber_primitive::UberPrimitive,
        },
    },
};
use egui::DragValue;
use egui_dnd::DragableItem;
use glam::{Vec2, Vec3, Vec4};

pub fn sphere_editor_ui(ui: &mut egui::Ui, sphere: &mut Sphere) -> EditState {
    sphere_editor_ui_fields(ui, &mut sphere.radius)
}

pub fn cube_editor_ui(ui: &mut egui::Ui, cube: &mut Cube) -> EditState {
    cube_editor_ui_fields(ui, &mut cube.dimensions)
}

pub fn uber_primitive_editor_ui(
    ui: &mut egui::Ui,
    uber_primitive: &mut UberPrimitive,
) -> EditState {
    uber_primitive_editor_ui_fields(
        ui,
        &mut uber_primitive.dimensions,
        &mut uber_primitive.corner_radius,
    )
}

pub fn primitive_transform_editor_ui(
    ui: &mut egui::Ui,
    primitive_transform: &mut PrimitiveTransform,
) -> EditState {
    let mut something_changed: bool = false;

    let center = &mut primitive_transform.center;
    ui.horizontal(|ui| {
        ui.label("Center:");
        something_changed |= ui
            .add(DragValue::new(&mut center.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.y).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.z).speed(DRAG_INC))
            .changed();
    });

    let rotation = &mut primitive_transform.rotation_tentative_append;
    todo!();
    ui.horizontal(|ui| {
        ui.label("Rotation:");
        something_changed |= ui
            .add(DragValue::new(&mut center.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.y).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut center.z).speed(DRAG_INC))
            .changed();
    });

    if something_changed {
        EditState::Modified
    } else {
        EditState::NoChange
    }
}

pub fn sphere_editor_ui_fields(ui: &mut egui::Ui, radius: &mut f32) -> EditState {
    let mut something_changed: bool = false;

    ui.horizontal(|ui| {
        ui.label("Radius:");
        something_changed |= ui
            .add(
                DragValue::new(radius)
                    .speed(DRAG_INC)
                    .clamp_range(0..=config::MAX_SPHERE_RADIUS),
            )
            .changed();
    });

    if something_changed {
        EditState::Modified
    } else {
        EditState::NoChange
    }
}

pub fn cube_editor_ui_fields(ui: &mut egui::Ui, dimensions: &mut Vec3) -> EditState {
    let mut something_changed: bool = false;

    ui.horizontal(|ui| {
        ui.label("Dimensions:");
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.y).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.z).speed(DRAG_INC))
            .changed();
    });

    if something_changed {
        EditState::Modified
    } else {
        EditState::NoChange
    }
}

pub fn uber_primitive_editor_ui_fields(
    ui: &mut egui::Ui,
    dimensions: &mut Vec4,
    corner_radius: &mut Vec2,
) -> EditState {
    let mut something_changed: bool = false;

    ui.horizontal(|ui| {
        ui.label("Dimensions:");
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.y).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.z).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut dimensions.w).speed(DRAG_INC))
            .changed();
    });

    ui.horizontal(|ui| {
        ui.label("Corner radius:");
        something_changed |= ui
            .add(DragValue::new(&mut corner_radius.x).speed(DRAG_INC))
            .changed();
        something_changed |= ui
            .add(DragValue::new(&mut corner_radius.y).speed(DRAG_INC))
            .changed();
    });

    if something_changed {
        EditState::Modified
    } else {
        EditState::NoChange
    }
}

impl DragableItem for PrimitiveOp {
    fn drag_id(&self) -> egui::Id {
        egui::Id::new(format!("p-op-drag{}", self.id()))
    }
}
