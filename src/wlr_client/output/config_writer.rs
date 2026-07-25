use core::f64;
use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use tracing::{debug, error, info, trace, warn};
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1,
    zwlr_output_configuration_v1::{Event, ZwlrOutputConfigurationV1},
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
};

use crate::commons::ToString;
use crate::wlr_client::errors::ClientError;
use crate::wlr_client::{ConfigurationsReadLock, ConnectionManager};

use super::wlr_head::OutputHead;
use crate::wlr_client::point::Point;

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
pub struct HeadUpdateRequest {
    pub head_id: u32,
    pub position: Option<Point>,
    pub mode_id: Option<u32>,
    pub scale: Option<f64>,
    pub rotation: Option<Transform>,
    pub enabled: Option<bool>,
}

impl Display for HeadUpdateRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HeadUpdateRequest: {}", self.head_id)?;
        if let Some(position) = self.position {
            write!(f, "\tposition: {position}")?;
        }
        if let Some(mode_id) = self.mode_id {
            write!(f, "\tmode: {mode_id}")?;
        }

        if let Some(scale) = self.scale {
            write!(f, "\tscale: {scale}")?;
        }

        if let Some(rotation) = self.rotation {
            write!(f, "\trotation: {rotation:?}")?;
        }

        if let Some(enabled) = self.enabled {
            write!(f, "\tenabled: {enabled}")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdateRequest {
    pub serial: u32,
    pub head_requests: Vec<HeadUpdateRequest>,
}

impl Display for UpdateRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "UpdateRequest: '{}'", self.serial)?;
        for req in &self.head_requests {
            writeln!(f, "\t{req}")?;
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
    queue_handle: QueueHandle<State>,
    connection: Connection,
    status_receiver: Receiver<Status>,
    shutdown: Arc<AtomicBool>,
}

impl ConfigWriter {
    pub(crate) fn new(conn_man: &ConnectionManager) -> Self {
        let queue: EventQueue<State> = conn_man.new_queue();
        let queue_handle = queue.handle();
        trace!("configurations writer queue created successfully");
        let (sender, receiver) = crossbeam_channel::unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));

        thread::spawn({
            let shutdown = shutdown.clone();
            let mut queue = queue;
            let mut state = State { sender };
            move || {
                loop {
                    if let Err(err) = queue.blocking_dispatch(&mut state) {
                        error!("Error dispatching config-writer events: {}", err);
                        break;
                    }
                    // shutdown polling thread on next wakeup rather than parking 
                    // on this connection forever.
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        });

        Self {
            queue_handle,
            connection: conn_man.connection(),
            status_receiver: receiver,
            shutdown,
        }
    }

    /// Handles sending the requests to the compositor to update the outputs configurations.
    pub fn write(
        &mut self,
        request: &UpdateRequest,
        output_manager: &ZwlrOutputManagerV1,
        configs_lock: &ConfigurationsReadLock,
    ) -> Result<(), ClientError> {
        debug!("received update request: {request}");
        let output_configuration =
            output_manager.create_configuration(request.serial, &self.queue_handle, ());
        trace!("output_configuration successfully created");
        let mut appy_please = false;
        for head_request in &request.head_requests {
            let configs = configs_lock.read();
            let head = configs
                .get_head(head_request.head_id)
                .ok_or("could not find head")?;
            let wlr_head = head.wlr_head();
            debug!("configuring head with name '{}'", head.name());
            if head_request.enabled == Some(false) {
                if head.enabled() {
                    debug!("disabling head '{}'", head.name());
                    output_configuration.disable_head(wlr_head);
                    appy_please = true;
                }
                continue;
            }

            let head_configuration =
                output_configuration.enable_head(wlr_head, &self.queue_handle, ());
            if Self::reconcile_head_configs(&head_configuration, configs_lock, head, head_request) {
                appy_please = true;
            }
        }
        if appy_please {
            info!("Applying new configurations");
            output_configuration.apply();
            // Flush ourselves: the dispatch thread is parked in `poll` and won't
            // flush this apply until it wakes — which only happens when compositor 
            // happens to send us some events.
            self.connection.flush()?;
            if let Ok(Status::Succeeded) = self.status_receiver.recv_timeout(Duration::from_secs(5))
            {
                info!("applied configurations successfully");
                return Ok(());
            }
            error!("failed to apply configurations");
            return Err(ClientError::general(String::from(
                "failed to update configurations",
            )));
        }
        Ok(())
    }

    fn reconcile_head_configs(
        head_configuration: &ZwlrOutputConfigurationHeadV1,
        configs_lock: &ConfigurationsReadLock,
        head: &OutputHead,
        request: &HeadUpdateRequest,
    ) -> bool {
        let mut dirty = false;

        if request.enabled == Some(true) && !head.enabled() {
            debug!("Head '{}' is being enabled", head.name());
            dirty = true;
        }

        if let Some(position) = request.position
            && head.position() != position
        {
            debug!(
                "Changes in position detected for head '{}' to {}",
                head.name(),
                position
            );
            head_configuration.set_position(position.0, position.1);
            dirty = true;
        }

        if let Some(mode_id) = request.mode_id {
            let configs = configs_lock.read();
            let mode = configs
                .get_mode(mode_id)
                .expect("Requested mode could not be found");
            if head.current_mode_id() != mode.wl_id() {
                debug!(
                    "Changes in mode detected for head '{}' to {:?}",
                    head.name(),
                    mode_id
                );
                head_configuration.set_mode(mode.wlr_mode());
                dirty = true;
            }
        }

        if let Some(scale) = request.scale
            && (head.scale() - scale).abs() > f64::EPSILON
        {
            debug!(
                "Changes in scale detected for head '{}' to {}",
                head.name(),
                scale
            );
            head_configuration.set_scale(scale);
            dirty = true;
        }

        if let Some(transform) = request.rotation
            && head.transform() != transform
        {
            debug!(
                "Changes in rotation detected for head '{}' to {}",
                head.name(),
                transform.to_string()
            );
            head_configuration.set_transform(transform);
            dirty = true;
        }

        dirty
    }
}

impl Drop for ConfigWriter {
    fn drop(&mut self) {
        // Shutdown the polling thread. not needed anymore
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
