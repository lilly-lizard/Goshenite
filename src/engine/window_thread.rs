use super::engine_controller::EngineCommand;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use single_value_channel::Updater;
use std::sync::mpsc::{self, Sender, TryRecvError};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

pub struct WindowThread {
    pub primary_window_id: WindowId,
    pub engine_command_tx: Updater<Option<EngineCommand>>,
    pub window_event_tx: Sender<WindowEvent>,
}

impl ApplicationHandler for WindowThread {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if event == WindowEvent::CloseRequested && window_id == self.primary_window_id {
            let _ = self.engine_command_tx.update(Some(EngineCommand::Quit));
            info!("close requested by window. stopping main thread...");
            event_loop.exit();

            return;
        }

        // send os event to engine thread
        let send_res = self.window_event_tx.send(event.clone());

        // handle premature engine closure
        if let Err(_e) = send_res {
            info!("engine thread disconnected. stopping main thread...");
            event_loop.exit();
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {}
}

pub struct WindowThreadChannels {
    /// FIFO queue
    pub engine_command_rx: single_value_channel::Receiver<Option<EngineCommand>>,
    pub window_event_rx: mpsc::Receiver<WindowEvent>,
}

impl WindowThreadChannels {
    /// Ordered by time received, i.e. first event in index 0
    pub fn get_events(&self) -> anyhow::Result<Vec<WindowEvent>> {
        let mut events = Vec::<WindowEvent>::new();
        loop {
            let recv_res = self.window_event_rx.try_recv();
            match recv_res {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => anyhow::bail!("window thread disconnected"),
            };
        }
        Ok(events)
    }

    pub fn latest_command(&mut self) -> Option<EngineCommand> {
        *self.engine_command_rx.latest()
    }
}
