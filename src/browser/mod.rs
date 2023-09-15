pub mod bridge;
pub mod control;

use std::{
    ffi::{c_char, c_float, c_int, c_void},
    ptr::null,
    slice::from_raw_parts,
    sync::{Arc, RwLock},
};

use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    runtime::Handle,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

use crate::{
    app::RawApp,
    ptr::{from_c_str, release_c_str, to_c_str, AsCStr},
    ActionState, ImeAction, Modifiers, MouseAction, TouchEventType, TouchPointerType,
};

use self::{
    bridge::{Bridge, BridgeObserver, BridgeOnContext, BridgeOnHandler},
    control::{Control, Rect},
};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BrowserState {
    Load = 1,
    LoadError = 2,
    BeforeLoad = 3,
    BeforeClose = 4,
    Close = 5,
}

#[repr(C)]
struct Ret {
    success: *const c_char,
    failure: *const c_char,
}

type BridgeOnCallback = extern "C" fn(callback_ctx: *mut c_void, ret: Ret);

#[repr(C)]
#[derive(Clone, Copy)]
struct RawBrowserObserver {
    on_state_change: extern "C" fn(state: BrowserState, ctx: *mut c_void),
    on_ime_rect: extern "C" fn(rect: Rect, ctx: *mut c_void),
    on_frame: extern "C" fn(buf: *const c_void, width: c_int, height: c_int, ctx: *mut c_void),
    on_title_change: extern "C" fn(title: *const c_char, ctx: *mut c_void),
    on_fullscreen_change: extern "C" fn(fullscreen: bool, ctx: *mut c_void),
    on_bridge: extern "C" fn(
        req: *const c_char,
        ctx: *mut c_void,
        callback_ctx: *mut c_void,
        callback: BridgeOnCallback,
    ),
}

#[repr(C)]
struct RawBrowserSettings {
    url: *const c_char,
    window_handle: *const c_void,
    frame_rate: u32,
    width: u32,
    height: u32,
    device_scale_factor: c_float,
    is_offscreen: bool,
}

impl Drop for RawBrowserSettings {
    fn drop(&mut self) {
        release_c_str(self.url);
    }
}

#[repr(C)]
pub(crate) struct RawBrowser {
    r#ref: *const c_void,
}

extern "C" {
    fn create_browser(
        app: *const RawApp,
        settings: *const RawBrowserSettings,
        observer: RawBrowserObserver,
        ctx: *mut c_void,
    ) -> *const RawBrowser;
    fn browser_exit(browser: *const RawBrowser);
    fn browser_resize(browser: *const RawBrowser, width: c_int, height: c_int);
    fn browser_get_hwnd(browser: *const RawBrowser) -> *const c_void;
}

#[derive(Debug, Clone, Copy)]
pub struct HWND(pub *const c_void);

unsafe impl Send for HWND {}
unsafe impl Sync for HWND {}

impl HWND {
    pub fn as_ptr(&self) -> *const c_void {
        self.0
    }

    pub fn is_null(&self) -> bool {
        !self.0.is_null()
    }
}

impl Default for HWND {
    fn default() -> Self {
        Self(null())
    }
}

#[derive(Debug)]
pub struct BrowserSettings<'a> {
    pub url: &'a str,
    pub window_handle: HWND,
    pub frame_rate: u32,
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f32,
    pub is_offscreen: bool,
}

impl Into<RawBrowserSettings> for &BrowserSettings<'_> {
    fn into(self) -> RawBrowserSettings {
        RawBrowserSettings {
            url: to_c_str(self.url),
            window_handle: self.window_handle.0,
            frame_rate: self.frame_rate,
            width: self.width,
            height: self.height,
            device_scale_factor: self.device_scale_factor,
            is_offscreen: self.is_offscreen,
        }
    }
}

#[allow(unused)]
pub trait Observer: Send + Sync {
    fn on_state_change(&self, state: BrowserState) {}
    fn on_ime_rect(&self, rect: Rect) {}
    fn on_frame(&self, texture: &[u8], width: u32, height: u32) {}
    fn on_title_change(&self, title: String) {}
    fn on_fullscreen_change(&self, fullscreen: bool) {}
}

enum ChannelEvents {
    StateChange(BrowserState),
}

#[derive(Clone)]
struct Delegation {
    observer: Arc<dyn Observer>,
    tx: Arc<UnboundedSender<ChannelEvents>>,
    on_bridge_callback: Arc<RwLock<Option<BridgeOnContext>>>,
}

impl Delegation {
    fn new<T>(observer: T) -> (Self, UnboundedReceiver<ChannelEvents>)
    where
        T: Observer + 'static,
    {
        let (tx, rx) = unbounded_channel();
        (
            Self {
                on_bridge_callback: Arc::new(RwLock::new(None)),
                observer: Arc::new(observer),
                tx: Arc::new(tx),
            },
            rx,
        )
    }
}

pub struct Browser {
    runtime: Handle,
    delegation: Delegation,
    delegation_ptr: *mut Delegation,
    settings: *mut RawBrowserSettings,
    ptr: *const RawBrowser,
}

unsafe impl Send for Browser {}
unsafe impl Sync for Browser {}

impl Browser {
    pub(crate) async fn new<T>(
        app: *const RawApp,
        settings: &BrowserSettings<'_>,
        observer: T,
    ) -> Result<Arc<Self>>
    where
        T: Observer + 'static,
    {
        let settings = Box::into_raw(Box::new(settings.into()));
        let (delegation, mut receiver) = Delegation::new(observer);
        let delegation_ptr = Box::into_raw(Box::new(delegation.clone()));
        let ptr =
            unsafe { create_browser(app, settings, BROWSER_OBSERVER, delegation_ptr as *mut _) };

        let (created_tx, created_rx) = oneshot::channel::<bool>();
        let delegation_ = delegation.clone();
        tokio::spawn(async move {
            let mut tx = Some(created_tx);
            while let Some(events) = receiver.recv().await {
                match events {
                    ChannelEvents::StateChange(state) => {
                        delegation_.observer.on_state_change(state);
                        match state {
                            BrowserState::LoadError => {
                                tx.take().map(|tx| tx.send(false));
                            }
                            BrowserState::Load => {
                                tx.take().map(|tx| tx.send(true));
                            }
                            _ => (),
                        }
                    }
                }
            }
        });

        if !created_rx.await? {
            return Err(anyhow!("create browser failed, maybe is load failed!"));
        }

        Ok(Arc::new(Self {
            runtime: Handle::current(),
            delegation_ptr,
            delegation,
            settings,
            ptr,
        }))
    }

    pub async fn call_bridge<Q, S>(&self, req: &Q) -> Result<Option<S>>
    where
        Q: Serialize,
        S: DeserializeOwned,
    {
        Bridge::call(self.ptr, req).await
    }

    pub fn on_bridge<Q, S, H>(&self, observer: H)
    where
        Q: DeserializeOwned + Send + 'static,
        S: Serialize + 'static,
        H: BridgeObserver<Req = Q, Res = S> + 'static,
    {
        let runtime = self.runtime.clone();
        let prcesser = Arc::new(BridgeOnHandler::new(observer));
        let _ = self
            .delegation
            .on_bridge_callback
            .write()
            .unwrap()
            .insert(BridgeOnContext(Arc::new(move |req, callback| {
                let prcesser = prcesser.clone();
                runtime.spawn(async move {
                    callback(prcesser.handle(&req).await);
                });
            })));
    }

    pub fn on_mouse(&self, action: MouseAction) {
        Control::on_mouse(self.ptr, action)
    }

    pub fn on_keyboard(&self, scan_code: u32, state: ActionState, modifiers: Modifiers) {
        Control::on_keyboard(self.ptr, scan_code, state, modifiers)
    }

    pub fn on_touch(
        &self,
        id: i32,
        x: i32,
        y: i32,
        ty: TouchEventType,
        pointer_type: TouchPointerType,
    ) {
        Control::on_touch(self.ptr, id, x, y, ty, pointer_type)
    }

    pub fn on_ime(&self, action: ImeAction) {
        Control::on_ime(self.ptr, action)
    }

    pub fn resize(&self, width: u32, height: u32) {
        unsafe { browser_resize(self.ptr, width as c_int, height as c_int) }
    }

    pub fn window_handle(&self) -> HWND {
        HWND(unsafe { browser_get_hwnd(self.ptr) })
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        unsafe { browser_exit(self.ptr) }
        drop(unsafe { Box::from_raw(self.delegation_ptr) });
        drop(unsafe { Box::from_raw(self.settings) });
    }
}

static BROWSER_OBSERVER: RawBrowserObserver = RawBrowserObserver {
    on_state_change,
    on_ime_rect,
    on_frame,
    on_title_change,
    on_fullscreen_change,
    on_bridge,
};

extern "C" fn on_state_change(state: BrowserState, ctx: *mut c_void) {
    (unsafe { &*(ctx as *mut Delegation) })
        .tx
        .send(ChannelEvents::StateChange(state))
        .expect("channel is closed, message send failed!");
}

extern "C" fn on_ime_rect(rect: Rect, ctx: *mut c_void) {
    (unsafe { &*(ctx as *mut Delegation) })
        .observer
        .on_ime_rect(rect);
}

extern "C" fn on_frame(texture: *const c_void, width: c_int, height: c_int, ctx: *mut c_void) {
    (unsafe { &*(ctx as *mut Delegation) }).observer.on_frame(
        unsafe { from_raw_parts(texture as *const _, width as usize * height as usize * 4) },
        width as u32,
        height as u32,
    );
}

extern "C" fn on_title_change(title: *const c_char, ctx: *mut c_void) {
    if let Some(title) = from_c_str(title) {
        (unsafe { &*(ctx as *mut Delegation) })
            .observer
            .on_title_change(title);
    }
}

extern "C" fn on_fullscreen_change(fullscreen: bool, ctx: *mut c_void) {
    (unsafe { &*(ctx as *mut Delegation) })
        .observer
        .on_fullscreen_change(fullscreen);
}

extern "C" fn on_bridge(
    req: *const c_char,
    ctx: *mut c_void,
    callback_ctx: *mut c_void,
    callback: BridgeOnCallback,
) {
    from_c_str(req).map(|req| {
        (unsafe { &*(ctx as *mut Delegation) })
            .on_bridge_callback
            .read()
            .unwrap()
            .as_ref()
            .map(|on_ctx| {
                let ctx = callback_ctx as usize;
                on_ctx.0.as_ref()(
                    req,
                    Box::new(move |ret| {
                        callback(
                            ctx as *mut c_void,
                            match ret {
                                Ok(res) => Ret {
                                    success: res.as_c_str().ptr,
                                    failure: null(),
                                },
                                Err(err) => Ret {
                                    failure: err.as_c_str().ptr,
                                    success: null(),
                                },
                            },
                        );
                    }),
                );
            });
    });
}
