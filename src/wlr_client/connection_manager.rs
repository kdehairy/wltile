use std::{
    ops::RangeInclusive,
    os::fd::OwnedFd,
    sync::{Arc, Condvar, Mutex},
    thread::{self},
    time::Duration,
};

use tracing::{error, trace};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    backend::{Backend, ObjectData, ObjectId, protocol},
    globals::{GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_display,
        wl_registry::{self},
    },
};

use crate::wlr_client::errors::ClientError;

/// The main function for `ConnectionManager` is to manage bindings to global objects.
///
/// binding more than once to global objects always tears down the previous binding and only the
/// last is kept functioning. This needs a single point of responsibility to make sure that
/// different parts of the app do not step on each other's toes.
pub(crate) struct ConnectionManager {
    connection: Connection,
    globals: Arc<GlobalList>,
    bound_interfaces: Vec<String>,
}

impl ConnectionManager {
    /// Establishes the connection to wayland socket and starts the pulling thread.
    ///
    /// The pulling threads only pulls from the socket and queues the events in the relevant queues.
    /// For each queue, there should be a call to [`EventQueue::dispatch_pending`]
    pub(super) fn connect() -> Result<ConnectionManager, ClientError> {
        let connection = Connection::connect_to_env()?;
        let (globals, mut queue) = registry_queue_init::<Data>(&connection)?;
        trace!("wayland global objects received");
        #[cfg(debug_assertions)]
        {
            trace!("List of found globals:");
            globals.contents().with_list(|list| {
                for i in list {
                    trace!("{}: v{}", i.interface, i.version);
                }
            });
        }

        thread::spawn(move || {
            loop {
                match queue.blocking_dispatch(&mut Data {}) {
                    Ok(num) => trace!("Dispatched {} registry events", num),
                    Err(err) => {
                        error!("fatal: registry dispatch failed: {err}");
                        std::process::exit(1);
                    }
                }
            }
        });

        Ok(ConnectionManager {
            connection,
            globals: Arc::new(globals),
            bound_interfaces: Vec::default(),
        })
    }

    /// Sends a `Sync` request to wayland server and blocks waiting for all messages and requests
    /// to arrive.
    pub(super) fn sync(&self) -> Result<(), ClientError> {
        Self::sync_connection(&self.connection)
    }

    /// Returns a clone of the underlying [`Connection`] handle.
    pub(super) fn connection(&self) -> Connection {
        self.connection.clone()
    }

    /// Same as [`ConnectionManager::sync`] but operates on a bare [`Connection`] handle.
    pub(super) fn sync_connection(conn: &Connection) -> Result<(), ClientError> {
        trace!("syncing with wayland server");
        let state = Arc::new(SyncData::default());
        let display = conn.display();

        conn.send_request(&display, wl_display::Request::Sync {}, Some(state.clone()))
            .map_err(|_| ClientError::Connection {
                msg: String::from("Failed to sync with wayland server"),
            })?;
        conn.flush()?;

        let done = state.done.lock().expect("sync mutex poisoned");
        let (_done, wait) = state
            .condvar
            .wait_timeout_while(done, Duration::from_secs(3), |done| !*done)
            .expect("sync mutex poisoned");
        if wait.timed_out() {
            error!("Timeout while syncing");
            return Err(ClientError::Dispatch {
                msg: String::from("Failed to sync with wayland server."),
            });
        }
        trace!("Syncing finished");

        Ok(())
    }

    pub(super) fn new_queue<State>(&self) -> EventQueue<State> {
        self.connection.new_event_queue()
    }

    /// Same as [`ConnectionManager::bind_global`] but instead of returning the first object of the requested
    /// interface it finds, it will return all objects.
    /// This is to return global objects for interfaces that has multiple like `wl_output`.
    pub(super) fn bind_all_globals<Iface, State, UData>(
        &mut self,
        queue_handle: &QueueHandle<State>,
        version: RangeInclusive<u32>,
        udata: UData,
    ) -> Result<Vec<Iface>, ClientError>
    where
        Iface: Proxy + 'static,
        State: Dispatch<Iface, UData> + 'static,
        UData: Clone + Send + Sync + 'static,
    {
        let version_start = *version.start();
        let version_end = *version.end();
        let interface = Iface::interface();

        // This is a panic because it's a compile-time programmer error, not a runtime error.
        assert!(
            version_end <= interface.version,
            "Maximum version ({}) of {} was higher than the proxy's maximum version ({}); outdated wayland XML files?",
            version.end(),
            interface.name,
            interface.version
        );

        let ifaces: Vec<(u32, u32)> = self.globals.contents().with_list(|guard| {
            guard
                .iter()
                .filter(|i| interface.name == &i.interface[..])
                .map(|i| (i.name, i.version))
                .collect()
        });

        let mut objects = Vec::new();
        for i in ifaces {
            if version_start < i.1 {
                return Err(ClientError::Binding {
                    msg: String::from("Min version constraint is not met"),
                });
            }
            objects.push(
                self.globals
                    .registry()
                    .bind(i.0, i.1, queue_handle, udata.clone()),
            );
        }

        Ok(objects)
    }

    /// Binds to global object.
    ///
    /// It makes sure that no one else has already did a binding with it.
    pub(super) fn bind_global<Iface, State, UData>(
        &mut self,
        queue_handle: &QueueHandle<State>,
        version: RangeInclusive<u32>,
        udata: UData,
    ) -> Result<Iface, ClientError>
    where
        Iface: Proxy + 'static,
        State: Dispatch<Iface, UData> + 'static,
        UData: Send + Sync + 'static,
    {
        let iface_name = Iface::interface().name.to_string();
        if self.bound_interfaces.contains(&iface_name) {
            error!(
                "Cannot allow second binding for global object '{}'",
                iface_name
            );
            return Err(ClientError::Binding {
                msg: String::from("Can't bind global object twice"),
            });
        }
        match self.globals.bind(queue_handle, version, udata) {
            Ok(interface) => {
                self.bound_interfaces
                    .push(String::from(interface.id().interface().name));
                Ok(interface)
            }
            Err(err) => Err(err)?,
        }
    }
}

struct Data {}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Data {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

struct SyncData {
    done: Mutex<bool>,
    condvar: Condvar,
}

impl Default for SyncData {
    fn default() -> Self {
        Self {
            done: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }
}

impl ObjectData for SyncData {
    fn event(
        self: Arc<Self>,
        _handle: &Backend,
        _msg: protocol::Message<ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        *self.done.lock().expect("sync mutex poisoned") = true;
        self.condvar.notify_all();
        None
    }

    fn destroyed(&self, _: ObjectId) {}
}
