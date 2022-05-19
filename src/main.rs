mod renderer;

pub use ash::{Device, Instance};
use renderer::Renderer;
use winit::{event_loop::EventLoop, window::WindowBuilder};

fn main() {
    let requested_width = 500;
    let requested_height = 500;

    // create winit window
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Ash - Example")
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(requested_width),
            f64::from(requested_height),
        ))
        .build(&event_loop)
        .unwrap();

    {
        // init renderer
        let renderer = Renderer::new(
            &window,
            "Goshenite Editor",
            1,
            requested_width,
            requested_height,
        );

        // start render loop
        renderer.render_loop(&mut event_loop);

        // render cleanup on drop
    }
}
