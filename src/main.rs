mod renderer;

use ash::{util::*, vk};
pub use ash::{Device, Instance};
use renderer::{find_memorytype_index, record_submit_commandbuffer, Renderer};
use std::{default::Default, ffi::CStr, io::Cursor, mem, mem::align_of};

fn main() {
    unsafe {
        let renderer = Renderer::new(100, 100);

        renderer.render_loop();
    }
}
