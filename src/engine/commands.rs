use glam::DVec3;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    SetCameraLockOn { target_pos: DVec3 },
    UnsetCameraLockOn,
    ResetCamera,
}
