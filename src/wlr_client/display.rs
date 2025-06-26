use std::os::fd::AsFd;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use tracing::{error, trace};
use wayland_client::WEnum;
use wayland_client::{
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_shm::{self, Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base;

use fontdue::{Font, FontSettings};
use wayland_protocols::xdg::shell::client::{xdg_surface::{self, XdgSurface}, xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase};

use crate::wlr_client::ConnectionManager;
use crate::wlr_client::errors::ClientError;
use crate::wlr_client::shmem::Shmem;

enum State {
    Wait,
    Ready,
}

pub struct DisplayServer {
    sh_mem: Shmem,
    wl_shm: WlShm,
    wl_pool: WlShmPool,
    wl_buff: WlBuffer,
    wl_surface: WlSurface,
    state: State,
    sender: Sender<State>,
    receiver: Receiver<State>,
    pixel_format: Option<wl_shm::Format>,
}

// We are assuming color format ARGB8888
const STRIDE: usize = 4; //bytes

impl DisplayServer {
    pub(super) fn start(
        conn_man: &mut ConnectionManager,
    ) -> Result<Self, ClientError> {

        let mut queue: EventQueue<DisplayServer> = conn_man.new_queue();
        let queue_handle = queue.handle();
        let wl_shm: WlShm = conn_man.bind_global(&queue_handle, 2..=2, ())?;
        let wl_compositor: WlCompositor = conn_man.bind_global(&queue_handle, 6..=6, ())?;
        let xdg_wm_base: XdgWmBase = conn_man.bind_global(&queue_handle, 5..=5, ())?;

        let (size, width, height) = estimate_shmem_size();
        let sh_mem: Shmem = Shmem::create(size).unwrap();
        let size = i32::try_from(size).unwrap();
        let wl_pool: WlShmPool = wl_shm.create_pool(sh_mem.fd.as_fd(), size, &queue_handle, ());
        let wl_buff = wl_pool.create_buffer(
            0,
            i32::try_from(width).unwrap(),
            i32::try_from(height).unwrap(),
            i32::try_from(STRIDE).unwrap(),
            Format::Argb8888,
            &queue_handle,
            (),
        );
        let wl_surface = wl_compositor.create_surface(&queue_handle, ());
        let xdg_surface = xdg_wm_base.get_xdg_surface(&wl_surface, &queue_handle, ());
        let _xdg_toplevel = xdg_surface.get_toplevel(&queue_handle, ());

        let (sender, receiver) = channel();

        let mut display_server = DisplayServer {
                    sh_mem,
                    wl_shm,
                    wl_pool,
                    wl_buff,
                    wl_surface,
                    state: State::Wait,
                    sender,
                    receiver,
                    pixel_format: None,
                };
        conn_man.sync()?;
        trace!("all is configured for display server");

        assert_eq!(display_server.pixel_format, Some(Format::Argb8888));

        Ok(display_server)
    }

    pub(crate) fn write(&self, text: &str) -> Result<(), String> {
        let (buff, ..) = rasterize_txt(text);
        if buff.len() > self.sh_mem.size {
            return Err(String::from("Pixel buffer is bigger than the display buffer"));
        }

        match self.receiver.recv_timeout(Duration::from_secs(10)) {
            Ok(State::Ready) => {
                unsafe {
                    std::ptr::copy(buff.as_ptr(), self.sh_mem.addr, buff.len());
                };
                self.wl_surface.attach(Some(&self.wl_buff), 0, 0);
                self.wl_surface.damage(0, 0, i32::MAX, i32::MAX);
                self.wl_surface.commit();
                Ok(())
            },
            Ok(State::Wait) => {
                error!("State did not change to Ready!");
                Err("Could not ack_configure!")? },
            Err(RecvTimeoutError::Timeout) => {
                error!("Timedout before receiving configure event.");
                Err("Failed to recieve state message.")?
            },
            Err(RecvTimeoutError::Disconnected) => {
                error!("Failed to receive state message.");
                Err("Failed to recieve state message.")?
            },
        }
    }
}

impl Drop for DisplayServer {
    fn drop(&mut self) {
        self.wl_surface.destroy();
        self.wl_buff.destroy();
        self.wl_pool.destroy();
        self.wl_shm.release();
    }
}

impl Dispatch<XdgWmBase, ()> for DisplayServer {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: <XdgWmBase as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<WlShm, ()> for DisplayServer {
    fn event(
        state: &mut Self,
        _proxy: &WlShm,
        event: <WlShm as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // We are guaranteed by wayland specifications that ARGB8888 and XRGB8888 must be supported
        // by the compositor.
        if let wl_shm::Event::Format {
            format: WEnum::Value(Format::Argb8888),
        } = event
        {
            state.pixel_format = Some(Format::Argb8888);
            trace!("pixel format 'ARGB8888' is supported");
        }
    }
}

impl Dispatch<WlCompositor, ()> for DisplayServer {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShmPool, ()> for DisplayServer {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        _event: <WlShmPool as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        todo!()
    }
}

impl Dispatch<WlBuffer, ()> for DisplayServer {
    fn event(
        _state: &mut Self,
        wl_buff: &WlBuffer,
        event: <WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_buffer::Event::Release = event {
            wl_buff.destroy();
        }
    }
}

impl Dispatch<WlSurface, ()> for DisplayServer {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // match event {
        //     wayland_client::protocol::wl_surface::Event::Enter { output } => {}
        //     wayland_client::protocol::wl_surface::Event::Leave { output } => {}
        //     wayland_client::protocol::wl_surface::Event::PreferredBufferScale { factor } => {}
        //     wayland_client::protocol::wl_surface::Event::PreferredBufferTransform { transform } => {
        //     }
        //     _ => {}
        // }
    }
}

impl Dispatch<XdgSurface, ()> for DisplayServer {
    fn event(
        state: &mut Self,
        proxy: &XdgSurface,
        event: <XdgSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            proxy.ack_configure(serial);
            state.state = State::Ready;
            if let Err(err) = state.sender.send(State::Ready) {
                error!("Failed to send the state message. {}", err);
            }
        }
    }
}

impl Dispatch<XdgToplevel, ()> for DisplayServer {
    fn event(
        _state: &mut Self,
        _proxy: &XdgToplevel,
        _event: <XdgToplevel as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // match event {
        //     xdg_toplevel::Event::Configure { width, height, states } => todo!(),
        //     xdg_toplevel::Event::Close => todo!(),
        //     xdg_toplevel::Event::ConfigureBounds { width, height } => todo!(),
        //     xdg_toplevel::Event::WmCapabilities { capabilities } => todo!(),
        //     _ => todo!(),
        // }
    }
}

fn rasterize_txt(txt: &str) -> (Vec<u8>, usize, usize) {
    let font_data: &[u8] = include_bytes!("DejaVuSans-Bold.ttf");
    let font = Font::from_bytes(
        font_data,
        FontSettings {
            scale: 360.0,
            ..Default::default()
        },
    )
    .unwrap();

    let mut px_h: usize = 0;
    let mut px_w: usize = 0;

    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::as_conversions
    )]
    for c in txt.chars() {
        let (metrics, _) = font.rasterize(c, 360.0);
        px_h = px_h.max(metrics.height);
        px_w = px_w.saturating_add(metrics.advance_width.abs().ceil() as usize);
    }

    let mut px_buff: Vec<u8> = vec![
        0u8;
        px_w.saturating_mul(px_h)
            .saturating_mul(px_w)
            .saturating_mul(STRIDE)
    ];

    let mut x_offset = 0;
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::as_conversions
    )]
    for c in txt.chars() {
        let (metrics, buff) = font.rasterize(c, 360.0);
        let ymin = usize::try_from(0.max(metrics.ymin)).unwrap();
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let src_idx = y.saturating_mul(metrics.width).saturating_add(x);
                let dst_idx = y
                    .saturating_add(px_h.saturating_sub(metrics.height).saturating_sub(ymin))
                    .saturating_mul(px_w)
                    .saturating_add(x_offset)
                    .saturating_add(x)
                    .saturating_mul(STRIDE);
                let grayscale = buff[src_idx];
                px_buff[dst_idx] = grayscale; // A
                px_buff[dst_idx.saturating_add(1)] = 0; // R
                px_buff[dst_idx.saturating_add(2)] = 0; // G
                px_buff[dst_idx.saturating_add(3)] = 0; // B
            }
        }
        x_offset = x_offset.saturating_add(metrics.advance_width.abs().ceil() as usize);
    }

    (px_buff, px_w, px_h)
}

fn estimate_shmem_size() -> (usize, usize, usize) {
    let (w, h) = {
        let font_data: &[u8] = include_bytes!("DejaVuSans-Bold.ttf");
        let font = Font::from_bytes(
            font_data,
            FontSettings {
                scale: 360.0,
                ..Default::default()
            },
        )
        .unwrap();
        let mut h: usize = 0;
        let mut w: usize = 0;
        for c in "HDMI-2".chars() {
            let (metrics, _) = font.rasterize(c, 360.0);
            h = if metrics.height > h {
                metrics.height
            } else {
                h
            };
            w = w.saturating_add(metrics.width);
        }
        (w, h)
    };
    (h.saturating_mul(w).saturating_mul(STRIDE), w, h)
}

#[cfg(test)]
mod tests {
    #[test]
    fn estimate_buffer_size() {
        let (size, _, _) = super::estimate_shmem_size();
        assert_eq!(1_230_656, size);
    }
}
