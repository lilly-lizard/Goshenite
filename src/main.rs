mod renderer;

pub use ash::{Device, Instance};
use renderer::Renderer;
use winit::{event_loop::EventLoop, window::WindowBuilder};

fn main() {
    let window_width = 100;
    let window_height = 100;

    // create winit window
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Ash - Example")
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(window_width),
            f64::from(window_height),
        ))
        .build(&event_loop)
        .unwrap();

    {
        // init renderer
        let renderer = Renderer::new(&window, window_width, window_height);

        // start render loop
        renderer.render_loop(&mut event_loop);

        // render cleanup on drop
    }
}
