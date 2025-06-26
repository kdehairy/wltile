use core::f64;
use std::fmt::Display;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use tracing::{debug, error, info, trace, warn};
use wayland_client::protocol::wl_output::Transform;
use wayland_client::Proxy;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1,
    zwlr_output_configuration_v1::{Event, ZwlrOutputConfigurationV1},
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
};

use crate::commons::ToString;
use crate::wlr_client::ConnectionManager;

use super::{point::Point, wlr_head::OutputHead};

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
                info!("Update was successful");
                if let Err(err) = state.sender.send(Status::Succeeded) {
                    error!("error sending status message: {err}");
                }
            }
            Event::Failed => {
                warn!("Update failed");
                if let Err(err) = state.sender.send(Status::Failed) {
                    error!("error sending status message: {err}");
                }
            }
            Event::Cancelled => {
                info!("Update was cancelled");
                if let Err(err) = state.sender.send(Status::Cancelled) {
                    error!("error sending status message: {err}");
                }
            }
            _ => error!("received undefined event from wayland compositor!"),
        }
        proxy.destroy();
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
    pub mode: Option<&'a ZwlrOutputModeV1>,
    pub scale: Option<f64>,
    pub rotation: Option<Transform>,
}

impl Display for HeadUpdateRequest<'_> {
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

impl Display for UpdateRequest<'_> {
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

/// Handles passing the desired configurations to the compositor.
pub struct ConfigWriter {
    queue: EventQueue<State>,
    queue_handle: QueueHandle<State>,
    state: State,
    status_receiver: Receiver<Status>,
}

impl ConfigWriter {
    pub(super) fn new(conn_man: &ConnectionManager) -> Self {
        let queue = conn_man.new_queue();
        trace!("configurations writer queue created successfully");
        let (sender, receiver) = channel();
        Self {
            queue_handle: queue.handle(),
            queue,
            state: State { sender },
            status_receiver: receiver,
        }
    }

    /// Handles sending the requests to the compositor to update the outputs configurations.
    pub fn write(
        &mut self,
        request: &UpdateRequest,
        output_manager: &ZwlrOutputManagerV1,
    ) -> Result<(), String> {
        let output_configuration =
            output_manager.create_configuration(request.serial, &self.queue_handle, ());
        debug!("output_configuration successfully created");
        let mut appy_please = false;
        for head_request in &request.head_requests {
            let (head, wlr_head) = (head_request.head, head_request.head.wlr_head());
            debug!("configuring head with name '{}'", head_request.head.name());
            let head_configuration =
                output_configuration.enable_head(wlr_head, &self.queue_handle, ());
            if Self::reconcile_head_configs(&head_configuration, head, head_request) {
                appy_please = true;
            }
        }
        if appy_please {
            info!("Applying new configurations");
            output_configuration.apply();
            let result;
            if let Err(err) = self.queue.roundtrip(&mut self.state) {
                result = Err(format!("error sending request to compositor: {err}"));
            } else if let Ok(Status::Succeeded) = self.status_receiver.recv_timeout(Duration::new(5, 0)) {
                result = Ok(());
            } else {
                result = Err(String::from("failed to update configurations"));
            }
            output_configuration.destroy();
            result
        } else {
            output_configuration.destroy();
            Ok(())
        }
    }

    fn reconcile_head_configs(
        head_configuration: &ZwlrOutputConfigurationHeadV1,
        head: &OutputHead,
        request: &HeadUpdateRequest,
    ) -> bool {
        let mut dirty = false;

        if let Some(position) = request.position {
            if *head.position() != position {
                debug!(
                    "Changes in position detected for head '{}' to {}",
                    head.name(),
                    position
                );
                head_configuration.set_position(position.0, position.1);
                dirty = true;
            }
        }

        if let Some(mode) = request.mode {
            if *head.current_mode_id() != mode.id() {
                debug!(
                    "Changes in mode detected for head '{}' to {:?}",
                    head.name(),
                    mode
                );
                head_configuration.set_mode(mode);
                dirty = true;
            }
        }

        if let Some(scale) = request.scale {
            if (head.scale() - scale).abs() > f64::EPSILON {
                debug!(
                    "Changes in scale detected for head '{}' to {}",
                    head.name(),
                    scale
                );
                head_configuration.set_scale(scale);
                dirty = true;
            }
        }

        if let Some(transform) = request.rotation {
            if head.transform() != transform {
                debug!(
                    "Changes in rotation detected for head '{}' to {}",
                    head.name(),
                    transform.to_string()
                );
                head_configuration.set_transform(transform);
                dirty = true;
            }
        }

        dirty
    }
}
