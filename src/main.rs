use std::collections::HashMap;

use wayland_client::protocol::wl_output::{self};
use wayland_client::protocol::wl_registry::{self};
use wayland_client::{Connection, Dispatch};

pub(crate) struct AppData {
    pub(crate) output_objs: HashMap<u32, wl_output::WlOutput>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == "wl_output" {
                    let out_proxy = proxy.bind(name, version, qhandle, ());
                    state.output_objs.insert(name, out_proxy);
                } else {
                    println!("[{}] {} (v{})", name, interface, version);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(out_obj) = state.output_objs.remove(&name) {
                    out_obj.release();
                }
            }
            _ => (),
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        event: <wl_output::WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_output::Event::Geometry {
            x,
            y,
            physical_width,
            physical_height,
            subpixel: _,
            make,
            model,
            transform: _,
        } = event
        {
            println!(
                "'{} {}' {} x {}: ({},{})",
                make, model, physical_height, physical_width, x, y
            );
        }
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let handle = event_queue.handle();
    let _ = display.get_registry(&handle, ());
    let mut app_data = AppData {
        output_objs: HashMap::new(),
    };
    event_queue.roundtrip(&mut app_data).unwrap();

    if app_data.output_objs.len() > 0 {
        event_queue.roundtrip(&mut app_data).unwrap();
    }
}
