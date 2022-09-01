use crate::camera::Camera;
use crate::config;
use crate::renderer::render_manager::RenderManager;
use std::sync::Arc;
use winit::event_loop::EventLoop;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder},
};

// Members
pub struct Controller {
    window: Arc<Window>,
    event_loop: EventLoop<()>,
    renderer: RenderManager,
    camera: Camera,
}

// Public functions
impl Controller {
    pub fn init() -> Self {
        // todo how default res usually handled?
        let init_resolution = [1000, 700];

        // create winit window
        let event_loop = EventLoop::new();
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(config::ENGINE_NAME)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    f64::from(init_resolution[0]),
                    f64::from(init_resolution[1]),
                ))
                .build(&event_loop)
                .unwrap(),
        );

        // init camera
        let camera = Camera::new(init_resolution);

        // init renderer
        let renderer = RenderManager::new(window.clone());

        Controller {
            window,
            event_loop,
            renderer,
            camera,
        }
    }

    pub fn start(&mut self) {
        let mut window_resize: bool = false;
        self.event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::WindowEvent {
                    event:
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    window_resize = true;
                    self.camera
                        .set_aspect_ratio(self.window.inner_size().into())
                }
                Event::MainEventsCleared => self.renderer.render_frame(window_resize, self.camera),
                Event::RedrawEventsCleared => window_resize = false,
                _ => (),
            }
        });
    }
}
