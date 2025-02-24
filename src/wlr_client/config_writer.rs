use std::fmt::Display;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1,
    zwlr_output_configuration_v1::{Event, ZwlrOutputConfigurationV1},
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
};

use super::{wlr_head::OutputHead, Point};

impl Dispatch<ZwlrOutputConfigurationV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputConfigurationV1,
        event: <ZwlrOutputConfigurationV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            Event::Succeeded => {
                log::info!("Update was successful");
                if let Err(err) = state.sender.send(Status::Succeeded) {
                    log::error!("error sending status message: {err}");
                }
                proxy.destroy();
            }
            Event::Failed => {
                log::warn!("Update failed");
                if let Err(err) = state.sender.send(Status::Failed) {
                    log::error!("error sending status message: {err}");
                }
                proxy.destroy();
            }
            Event::Cancelled => {
                log::info!("Update was cancelled");
                if let Err(err) = state.sender.send(Status::Cancelled) {
                    log::error!("error sending status message: {err}");
                }
                proxy.destroy();
            }
            _ => log::error!("received undefined event from wayland compositor!"),
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        _event: <ZwlrOutputConfigurationHeadV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

#[derive(Debug)]
pub struct HeadUpdateRequest<'a> {
    pub head: &'a OutputHead,
    pub position: Option<Point>,
}

impl<'a> Display for HeadUpdateRequest<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Request for Head '{}' to position '{}'",
            self.head.name(),
            self.position.unwrap_or(*self.head.position())
        )
    }
}

#[derive(Debug)]
pub struct UpdateRequest<'a> {
    pub serial: u32,
    pub head_requests: Vec<HeadUpdateRequest<'a>>,
}

impl<'a> Display for UpdateRequest<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "update request with serial: '{}' and updates:",
            self.serial
        )?;
        for req in &self.head_requests {
            writeln!(f, "  - {req}")?;
        }
        Ok(())
    }
}

struct State {
    sender: Sender<Status>,
}

#[derive(PartialEq)]
enum Status {
    Succeeded,
    Failed,
    Cancelled,
}

pub struct ConfigWriter {
    queue: EventQueue<State>,
    queue_handle: QueueHandle<State>,
    state: State,
    status_receiver: Receiver<Status>,
}

impl ConfigWriter {
    pub fn new(wlr_connection: &Connection) -> Self {
        let queue = wlr_connection.new_event_queue();
        let (sender, receiver) = channel();
        log::debug!("queue created successfully");
        Self {
            queue_handle: queue.handle(),
            queue,
            state: State { sender },
            status_receiver: receiver,
        }
    }

    pub fn write(&mut self, request: &UpdateRequest, output_manager: &ZwlrOutputManagerV1) -> bool {
        let output_configuration =
            output_manager.create_configuration(request.serial, &self.queue_handle, ());
        let mut appy_please = false;
        for head_request in &request.head_requests {
            let (head, wlr_head) = (head_request.head, head_request.head.wlr_head());
            log::debug!("Found head with name '{}'", head_request.head.name());
            let head_configuration =
                output_configuration.enable_head(wlr_head, &self.queue_handle, ());
            if Self::reconcile_head_configs(&head_configuration, head, head_request) {
                appy_please = true;
            }
        }
        if appy_please {
            log::info!("Applying new configurations");
            output_configuration.apply();
            self.queue.roundtrip(&mut self.state).unwrap();
            if let Ok(status) = self.status_receiver.recv_timeout(Duration::new(5, 0)) {
                return status == Status::Succeeded;
            }
            return false;
        }
        true
    }

    fn reconcile_head_configs(
        head_configuration: &ZwlrOutputConfigurationHeadV1,
        head: &OutputHead,
        request: &HeadUpdateRequest,
    ) -> bool {
        if let Some(position) = request.position.as_ref() {
            if head.position() != position {
                log::debug!(
                    "Changes in position detected for head '{}' to {}",
                    head.name(),
                    position
                );
                head_configuration.set_position(position.0, position.1);
                return true;
            }
        }
        false
    }
}
