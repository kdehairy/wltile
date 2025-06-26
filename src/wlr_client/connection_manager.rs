use std::{
    ops::RangeInclusive,
    os::fd::OwnedFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{self, Duration},
};

use tracing::{error, trace};
use wayland_client::{
    backend::{protocol, Backend, ObjectData, ObjectId},
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{wl_display, wl_registry},
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};

use crate::wlr_client::errors::ClientError;

pub(super) struct ConnectionManager {
    connection: Connection,
    globals: GlobalList,
    bound_interfaces: Vec<String>,
}

impl ConnectionManager {
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

        thread::spawn(move || loop {
            // We don't really care about this specific queue.
            // We are invoking it to force the connection to read events out of the socket.
            match queue.blocking_dispatch(&mut Data {}) {
                Ok(num) => trace!("Dispatched {} events", num),
                Err(err) => {
                    error!("Failed to dispatch events: {}", err);
                    return;
                }
            }
        });

        Ok(ConnectionManager {
            connection,
            globals,
            bound_interfaces: Vec::default(),
        })
    }

    pub(super) fn sync(&self) -> Result<(), ClientError> {
        trace!("syncing with wayland server");
        let conn = &self.connection;
        let done = Arc::new(SyncData::default());
        let display = conn.display();

        conn.send_request(&display, wl_display::Request::Sync {}, Some(done.clone()))
            .map_err(|_| ClientError::Connection {
                msg: String::from("Failed to sync with wayland server"),
            })?;
        conn.flush()?;

        let end = time::SystemTime::now()
            .checked_add(Duration::from_secs(3))
            .expect("Should not happen");
        loop {
            if end <= time::SystemTime::now() {
                error!("Timeout while syncing");
                break;
            }
            // see if the successful read included our callback
            if done.done.load(Ordering::Relaxed) {
                break;
            }
        }
        trace!("Syncing finished");

        Ok(())
    }

    pub(super) fn new_queue<State>(&self) -> EventQueue<State> {
        self.connection.new_event_queue()
    }

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

#[derive(Default)]
struct SyncData {
    done: AtomicBool,
}

impl ObjectData for SyncData {
    fn event(
        self: Arc<Self>,
        _handle: &Backend,
        _msg: protocol::Message<ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        self.done.store(true, Ordering::Relaxed);
        None
    }

    fn destroyed(&self, _: ObjectId) {}
}
