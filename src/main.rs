mod renderer;

pub use ash::{Device, Instance};
use renderer::Renderer;

fn main() {
    let renderer = Renderer::new(100, 100);

    renderer.render_loop();
}
