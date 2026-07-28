use std::collections::HashMap;
use std::os::fd::AsFd;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use tracing::{debug, error, trace};
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_surface;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_shm::{self, Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};
use wayland_client::{Proxy, WEnum};
use wayland_protocols::xdg::shell::client::{xdg_toplevel, xdg_wm_base};

use fontdue::{Font, FontSettings};
use wayland_protocols::xdg::shell::client::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::XdgToplevel,
    xdg_wm_base::XdgWmBase,
};

use crate::wlr_client::errors::ClientError;
use crate::wlr_client::output::heads::Head;
use crate::wlr_client::output::wlr_head::OutputHead;
use crate::wlr_client::shmem::Shmem;
use crate::wlr_client::{ConfigurationsReadLock, ConnectionManager};

#[derive(Debug, PartialEq)]
enum State {
    /// frame buffer is all set and ready to go.
    Ready,
    /// frame buffer committed to the compositor.
    Committed,
    /// frame buffer is displayed and is desposed by the compositor already. Could be re-used at
    /// this state.
    Active,
}

// We need to wrap the state in our own type to be able to impl alien traits.
struct StateWrapper {
    state: AsyncState,
}

struct DisplayState {
    format: Option<wl_shm::Format>,
    surfaces: Vec<Surface>,
    outputs_map: HashMap<String, ObjectId>,
    outputs: Vec<WlOutput>,
}

#[allow(clippy::struct_field_names)]
struct Surface {
    wl_surface: WlSurface,
    wl_buff: WlBuffer,
    sh_mem_offset: isize,
    xdg_surface: XdgSurface,
    xdg_toplevel: XdgToplevel,
    state: State,
    output: Option<WlOutput>,
    transform: Option<WEnum<wl_output::Transform>>,
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.xdg_toplevel.destroy();
        self.xdg_surface.destroy();
        self.wl_surface.destroy();
        self.wl_buff.destroy();
    }
}

type AsyncState = Arc<RwLock<DisplayState>>;

pub struct DisplayServer {
    queue_handle: QueueHandle<StateWrapper>,
    sh_mem: Shmem,
    sh_mem_offset: isize,
    wl_shm: WlShm,
    wl_pool: WlShmPool,
    wl_compositor: WlCompositor,
    xdg_wm_base: XdgWmBase,
    state: AsyncState,
    configs_lock: ConfigurationsReadLock,
    connection: Connection,
}

// We are assuming color format ARGB8888
const PIXEL_SIZE: usize = 4; //bytes
const FONT_SIZE: f32 = 128.0;
const BACKGROUND: u8 = 128;
const BORDER_SIZE: usize = 25;

impl DisplayServer {
    pub(super) fn start(
        conn_man: &mut ConnectionManager,
        configs_lock: ConfigurationsReadLock,
    ) -> Result<Self, ClientError> {
        let mut queue: EventQueue<StateWrapper> = conn_man.new_queue();
        let queue_handle = queue.handle();
        let wl_shm: WlShm = conn_man.bind_global(&queue_handle, 2..=2, ())?;
        let wl_compositor: WlCompositor = conn_man.bind_global(&queue_handle, 6..=6, ())?;
        let xdg_wm_base: XdgWmBase = conn_man.bind_global(&queue_handle, 5..=5, ())?;

        let wl_outputs: Vec<WlOutput> = conn_man.bind_all_globals(&queue_handle, 4..=4, ())?;

        let (buff, ..) = rasterize_txt("HDMI-2HDMI-2".repeat(wl_outputs.len()).as_str());
        debug!("Estemated display buffer: size: {}", buff.len());
        let sh_mem: Shmem = Shmem::create(buff.len()).unwrap();
        let size = i32::try_from(buff.len()).unwrap();
        let wl_pool: WlShmPool = wl_shm.create_pool(sh_mem.fd.as_fd(), size, &queue_handle, ());

        let display_state = Arc::new(RwLock::new(DisplayState {
            format: None,
            surfaces: Vec::default(),
            outputs_map: HashMap::default(),
            outputs: wl_outputs,
        }));

        // let's sync with the server and dispatch all pending events after. This will make sure
        // we got the supported pixel formats.
        conn_man.sync()?;
        queue.dispatch_pending(&mut StateWrapper {
            state: display_state.clone(),
        })?;

        thread::spawn({
            let mut state = StateWrapper {
                state: display_state.clone(),
            };

            move || loop {
                if let Err(err) = queue.blocking_dispatch(&mut state) {
                    error!("fatal: display dispatch failed: {err}");
                    std::process::exit(1);
                }
            }
        });

        let display_server = DisplayServer {
            queue_handle,
            sh_mem,
            sh_mem_offset: 0,
            wl_shm,
            wl_pool,
            wl_compositor,
            xdg_wm_base,
            state: display_state,
            configs_lock,
            connection: conn_man.connection(),
        };
        debug!("all is configured for display server");

        Ok(display_server)
    }

    pub(crate) fn write(&mut self, text: &str, head: Option<&Head>) -> Result<(), ClientError> {
        let (buff, width, height) = rasterize_txt(text);
        if buff.len() > self.sh_mem.size {
            error!(
                "Display buffer of size '{}' is too small for pixel buffer",
                self.sh_mem.size
            );
            return Err(ClientError::Display {
                msg: String::from("Pixel buffer is bigger than the display buffer"),
            });
        }
        debug!(
            "Pixel buffer size: {}, w:{}, h:{}",
            buff.len(),
            width,
            height
        );

        let mut stored_surface = None;
        let mut rw_lock = self.state.write().unwrap();
        if let Some(h) = head.as_ref()
            && let Some(o) = self.configs_lock.read().get_head(h.id())
            && let Some(wl_output) = rw_lock.wl_output_from_output_head(o)
        {
            for s in &mut rw_lock.surfaces {
                if let Some(so) = s.output.as_ref()
                    && so.id() == wl_output.id()
                {
                    stored_surface = Some(s);
                    break;
                }
            }
        }

        if let Some(s) = stored_surface {
            unsafe {
                std::ptr::copy(
                    buff.as_ptr(),
                    //FIXME: This is assuming that the same text (in size) is being displayed.
                    // If this assumption breaks the pixel buffer will overflow to the next.
                    // This is fine globally for the crate, since we are always writing the output
                    // name. But from the display module perspective, it does not know that.
                    self.sh_mem.addr.offset(s.sh_mem_offset),
                    buff.len(),
                );
            };
            s.state = State::Ready;
            drop(rw_lock);
        } else {
            drop(rw_lock);
            let mut wl_output = None;
            if let Some(h) = head.as_ref()
                && let Some(oh) = self.configs_lock.read().get_head(h.id())
            {
                wl_output = match self.state.read().unwrap().wl_output_from_output_head(oh) {
                    Some(o) => Some(o),
                    None => {
                        return Err(ClientError::Display {
                            msg: String::from("Could not find specified output"),
                        });
                    }
                };
            }

            // Offset into the shared pool where this surface's buffer will lives.
            let buffer_offset = self.sh_mem_offset;

            let wl_buff = self.wl_pool.create_buffer(
                i32::try_from(buffer_offset).unwrap(),
                i32::try_from(width).unwrap(),
                i32::try_from(height).unwrap(),
                i32::try_from(width.saturating_mul(PIXEL_SIZE)).unwrap(),
                Format::Argb8888,
                &self.queue_handle,
                (),
            );

            let wl_surface = self.wl_compositor.create_surface(&self.queue_handle, ());
            let xdg_surface = self
                .xdg_wm_base
                .get_xdg_surface(&wl_surface, &self.queue_handle, ());
            let xdg_toplevel = xdg_surface.get_toplevel(&self.queue_handle, ());

            unsafe {
                std::ptr::copy(
                    buff.as_ptr(),
                    //FIXME: We need to check first if from this offset forward we have enough space
                    // in the shmem.
                    // It's Ok from the whole crate perspective, since we are always going through
                    // the outputs once, and we allocate enough buffer for them all (and a bit more).
                    self.sh_mem.addr.offset(buffer_offset),
                    buff.len(),
                );
            }

            self.sh_mem_offset =
                buffer_offset.saturating_add(isize::try_from(buff.len()).unwrap());

            let commit_surface = wl_surface.clone();
            let fullscreen_toplevel = xdg_toplevel.clone();

            let surface = Surface {
                wl_surface,
                wl_buff,
                sh_mem_offset: buffer_offset,
                xdg_surface,
                xdg_toplevel,
                state: State::Ready,
                output: wl_output.clone(),
                transform: None,
            };

            // Register the surface BEFORE the role-assigning commit that makes
            // the compositor emit `xdg_surface::configure`. If we committed
            // first, the display thread could dispatch that configure before the
            // surface is tracked here — the handler would find nothing, the
            // buffer would never be attached, and the surface would stall in
            // `Ready` forever (blank output + `wait_until_presented` timeout).
            self.state.write().unwrap().surfaces.push(surface);

            // Now assign the top_level role and request the configure.
            commit_surface.commit();
            fullscreen_toplevel.set_fullscreen(wl_output.as_ref());

            // Flush these requests ourselves. The display thread now blocks in
            // `blocking_dispatch` and won't flush again until it wakes — but it
            // can't wake until the configure that this commit triggers arrives.
            // Without this flush the two would deadlock.
            self.connection.flush()?;
        }

        Ok(())
    }

    /// Blocks until every surface created so far has attached its buffer and committed it to the
    /// compositor (`State::Committed` or later), or until `timeout` elapses, then drives one
    /// roundtrip so the commit is guaranteed to have reached and been processed by the server.
    pub(crate) fn wait_until_presented(&self, timeout: Duration) {
        let deadline = Instant::now()
            .checked_add(timeout)
            .expect("timeout overflowed the clock");
        loop {
            let all_committed = self
                .state
                .read()
                .unwrap()
                .surfaces
                .iter()
                .all(|s| matches!(s.state, State::Committed | State::Active));
            if all_committed {
                break;
            }
            if Instant::now() >= deadline {
                debug!("Timed out waiting for surfaces to be presented");
                break;
            }
            thread::sleep(Duration::from_millis(4));
        }
        if let Err(err) = ConnectionManager::sync_connection(&self.connection) {
            error!("Failed to sync after committing surfaces: {}", err);
        }
    }
}

impl Drop for DisplayServer {
    fn drop(&mut self) {
        self.wl_pool.destroy();
        self.wl_shm.release();
    }
}

impl DisplayState {
    fn wl_output_from_output_head(&self, output_head: &OutputHead) -> Option<WlOutput> {
        let mut result = None;
        if let Some(object_id) = self.outputs_map.get(output_head.name()) {
            for output in &self.outputs {
                if output.id() == *object_id {
                    result = Some(output.clone());
                    break;
                }
            }
        }
        result
    }
}

impl Drop for DisplayState {
    fn drop(&mut self) {
        for output in &self.outputs {
            output.release();
        }
    }
}

impl Dispatch<WlOutput, ()> for StateWrapper {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            state
                .state
                .write()
                .unwrap()
                .outputs_map
                .insert(name, proxy.id());
        }
    }
}

impl Dispatch<XdgWmBase, ()> for StateWrapper {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: <XdgWmBase as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            trace!("pong({})", serial);
            proxy.pong(serial);
        }
    }
}

impl Dispatch<WlShm, ()> for StateWrapper {
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
            state.state.write().unwrap().format = Some(Format::Argb8888);
            debug!("pixel format 'ARGB8888' is supported");
        }
    }
}

impl Dispatch<WlCompositor, ()> for StateWrapper {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // match _event {
        //     _ => todo!(),
        // }
    }
}

impl Dispatch<WlShmPool, ()> for StateWrapper {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        _event: <WlShmPool as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        //match _event {
        //    _ => todo!(),
        //}
    }
}

impl Dispatch<WlBuffer, ()> for StateWrapper {
    fn event(
        state: &mut Self,
        wl_buff: &WlBuffer,
        event: <WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_buffer::Event::Release = event {
            debug!("buffer released by compositor");
            for surface in &mut state.state.write().unwrap().surfaces {
                if surface.wl_buff.id() == wl_buff.id() {
                    surface.state = State::Active;
                    debug!("surface {} is active", surface.wl_surface.id());
                    break;
                }
            }
        }
    }
}

impl Dispatch<WlSurface, ()> for StateWrapper {
    fn event(
        state: &mut Self,
        proxy: &WlSurface,
        event: <WlSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wl_surface::Event::PreferredBufferTransform { transform } = event {
            for surface in &mut state.state.write().unwrap().surfaces {
                if surface.wl_surface.id() == proxy.id() {
                    surface.transform = Some(transform);
                    break;
                }
            }
        }
    }
}

impl Dispatch<XdgSurface, ()> for StateWrapper {
    fn event(
        state: &mut Self,
        proxy: &XdgSurface,
        event: <XdgSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            debug!("received configure event on xdg_surface");
            proxy.ack_configure(serial);
            for surface in &mut state.state.write().unwrap().surfaces {
                if surface.xdg_surface.id() == proxy.id() && surface.state == State::Ready {
                    surface.wl_surface.attach(Some(&surface.wl_buff), 0, 0);
                    surface.wl_surface.commit();
                    surface.state = State::Committed;
                    break;
                }
            }
        }
    }
}

impl Dispatch<XdgToplevel, ()> for StateWrapper {
    fn event(
        _state: &mut Self,
        _proxy: &XdgToplevel,
        event: <XdgToplevel as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                debug!(
                    "xdg_top_level::configure({}, {}, {:?})",
                    width, height, states
                );
            }
            //xdg_toplevel::Event::Close => {}
            xdg_toplevel::Event::ConfigureBounds { width, height } => {
                debug!("xdg_top_level::configure_bounds({}, {})", width, height);
            }
            xdg_toplevel::Event::WmCapabilities { capabilities } => {
                debug!("xdg_top_level::wm_capabilities({:?})", capabilities);
            }
            _ => {}
        }
    }
}

fn rasterize_txt(txt: &str) -> (Vec<u8>, usize, usize) {
    let font_data: &[u8] = include_bytes!("DejaVuSans-Bold.ttf");
    let font = Font::from_bytes(
        font_data,
        FontSettings {
            scale: FONT_SIZE,
            ..Default::default()
        },
    )
    .unwrap();

    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::as_conversions
    )]
    // calculate the text width and height based on font metrics for each character
    let (txt_w, txt_h) = {
        let mut txt_h: usize = 0;
        let mut txt_w: usize = 0;
        for (i, c) in txt.chars().enumerate() {
            let (metrics, _) = font.rasterize(c, FONT_SIZE);
            txt_h = txt_h.max(metrics.height);
            let advance = if i < txt.len().saturating_sub(1) {
                metrics.advance_width.abs().ceil() as usize
            } else {
                metrics.width
            };
            txt_w = txt_w.saturating_add(advance);
        }
        (txt_w, txt_h)
    };

    let px_w = txt_w.saturating_add(BORDER_SIZE.saturating_mul(2));
    let px_h = txt_h.saturating_add(BORDER_SIZE.saturating_mul(2));
    let mut px_buff: Vec<u8> =
        vec![BACKGROUND; px_w.saturating_mul(px_h).saturating_mul(PIXEL_SIZE)];
    //ensures that all pixels are opaque
    for px in px_buff.chunks_exact_mut(4) {
        px[3] = 255;
    }

    let mut x_offset = BORDER_SIZE;
    let y_offset = BORDER_SIZE;

    // rendering the grayscale text pixel buffer in the middle of the overall buffer (with borders)
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::as_conversions
    )]
    for c in txt.chars() {
        let (metrics, buff) = font.rasterize(c, FONT_SIZE);
        let ymin = usize::try_from(0.max(metrics.ymin)).unwrap();
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let src_idx = y.saturating_mul(metrics.width).saturating_add(x);
                let dst_idx = y
                    .saturating_add(
                        px_h.saturating_sub(metrics.height)
                            .saturating_sub(ymin)
                            .saturating_sub(y_offset),
                    )
                    .saturating_mul(px_w)
                    .saturating_add(x_offset)
                    .saturating_add(x)
                    .saturating_mul(PIXEL_SIZE);
                let grayscale = buff[src_idx];
                // blinding with the background color.
                let px_color = f32::from(BACKGROUND) * (1.0 - (f32::from(grayscale) / 255.0));
                let px_color = px_color.ceil() as u8;
                px_buff[dst_idx] = px_color; // B
                px_buff[dst_idx.saturating_add(1)] = px_color; // G
                px_buff[dst_idx.saturating_add(2)] = px_color; // R
                px_buff[dst_idx.saturating_add(3)] = 255; // A
            }
        }
        x_offset = x_offset.saturating_add(metrics.advance_width.abs().ceil() as usize);
    }

    (px_buff, px_w, px_h)
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, RgbaImage};

    use crate::wlr_client::display::rasterize_txt;

    #[test]
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    fn rasterizing_txt() {
        let (buff, w, h) = rasterize_txt("HDMI-2");
        let mut rgba_buff = Vec::new();
        for pixel in buff.chunks_exact(4) {
            rgba_buff.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
        let img: RgbaImage =
            ImageBuffer::from_raw(w as u32, h as u32, rgba_buff).expect("don't fail");
        let _ = img.save("target/test_img.png");
    }
}
