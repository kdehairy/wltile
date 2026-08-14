use std::{
    sync::{Arc, RwLock},
    thread,
};

use crossbeam_channel::{Receiver, Sender};
use tracing::{error, trace};
use wayland_client::{
    Dispatch, EventQueue, QueueHandle, WEnum,
    protocol::{
        wl_keyboard::{KeyState, WlKeyboard},
        wl_seat::{self, Capability, WlSeat},
    },
};

use crate::wlr_client::{ConnectionManager, errors::ClientError};

type AsyncState = Arc<RwLock<KeyboardState>>;

struct StateWrapper {
    state: AsyncState,
}

struct KeyboardState {
    keyboard: Option<WlKeyboard>,
    key_pressed_tx: Sender<u32>,
}

pub(crate) struct InputServer {
    seat: WlSeat,
    state: AsyncState,
    key_pressed_rx: Receiver<u32>,
}

impl InputServer {
    pub(crate) fn start(conn_man: &mut ConnectionManager) -> Result<InputServer, ClientError> {
        let mut queue: EventQueue<StateWrapper> = conn_man.new_queue();
        let queue_handle = queue.handle();
        let seat: WlSeat = conn_man.bind_global(&queue_handle, 9..=9, ())?;

        let (tx, rx) = crossbeam_channel::unbounded();

        let state = Arc::new(RwLock::new(KeyboardState {
            keyboard: None,
            key_pressed_tx: tx,
        }));

        conn_man.sync()?;
        queue.dispatch_pending(&mut StateWrapper {
            state: state.clone(),
        })?;

        thread::spawn({
            let mut state = StateWrapper {
                state: state.clone(),
            };

            move || loop {
                if let Err(err) = queue.blocking_dispatch(&mut state) {
                    error!("fatal: input dispatch failed: {err}");
                    std::process::exit(1);
                }
            }
        });

        Ok(InputServer {
            seat,
            state,
            key_pressed_rx: rx,
        })
    }

    pub(crate) fn wait_for_user_input(&self) {
        let _ = self.key_pressed_rx.recv();
    }
}

impl Drop for InputServer {
    fn drop(&mut self) {
        if let Some(keyboard) = self.state.write().unwrap().keyboard.as_ref() {
            keyboard.release();
        }
        self.seat.release();
    }
}

impl Dispatch<WlSeat, ()> for StateWrapper {
    fn event(
        state: &mut Self,
        proxy: &WlSeat,
        event: <WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capability),
        } = event
        {
            let mut kstate = state.state.write().unwrap();
            if capability.contains(Capability::Keyboard) && kstate.keyboard.is_none() {
                kstate.keyboard = Some(proxy.get_keyboard(qhandle, ()));
            }
        }
    }
}

impl Dispatch<WlKeyboard, ()> for StateWrapper {
    fn event(
        state: &mut Self,
        _proxy: &WlKeyboard,
        event: <WlKeyboard as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_keyboard::Event::Key {
                key,
                state: key_state,
                ..
            } => {
                trace!("key: {}, {:?}", key, key_state);
                if let WEnum::Value(KeyState::Pressed) = key_state {
                    let _ = state.state.read().unwrap().key_pressed_tx.send(key);
                }
            }
            wayland_client::protocol::wl_keyboard::Event::Leave { .. } => {
                trace!("keyboard focus left; dismissing");
                let _ = state.state.read().unwrap().key_pressed_tx.send(0);
            }
            _ => {}
        }
    }
}
