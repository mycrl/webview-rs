use std::{
    ffi::{c_char, c_int, c_void},
    sync::Arc,
    thread,
};

use crate::{
    args_ptr,
    ptr::{opt_to_c_str, release_c_str, AsCStr},
    Browser, BrowserSettings, Observer,
};

use anyhow::{anyhow, Result};
use tokio::sync::{
    oneshot::{self, Sender},
    Notify,
};

#[repr(C)]
struct RawAppSettings {
    cache_path: *const c_char,
    browser_subprocess_path: *const c_char,
    scheme_path: *const c_char,
}

impl Drop for RawAppSettings {
    fn drop(&mut self) {
        release_c_str(self.cache_path);
        release_c_str(self.scheme_path);
        release_c_str(self.browser_subprocess_path);
    }
}

#[repr(C)]
pub(crate) struct RawApp {
    settings: *const RawAppSettings,
    r#ref: *const c_void,
}

type CreateAppCallback = extern "C" fn(ctx: *mut c_void);

extern "C" {
    fn create_app(
        settings: *const RawAppSettings,
        callback: CreateAppCallback,
        ctx: *mut c_void,
    ) -> *const RawApp;
    fn app_run(app: *const RawApp, argc: c_int, args: *const *const c_char) -> c_int;
    fn app_exit(app: *const RawApp);
}

#[derive(Debug)]
pub struct AppSettings<'a> {
    pub cache_path: Option<&'a str>,
    pub browser_subprocess_path: Option<&'a str>,
    pub scheme_path: Option<&'a str>,
}

impl Into<RawAppSettings> for &AppSettings<'_> {
    fn into(self) -> RawAppSettings {
        RawAppSettings {
            cache_path: opt_to_c_str(self.cache_path),
            scheme_path: opt_to_c_str(self.scheme_path),
            browser_subprocess_path: opt_to_c_str(self.browser_subprocess_path),
        }
    }
}

pub struct App {
    notify: Notify,
    settings: *mut RawAppSettings,
    ptr: *const RawApp,
}

unsafe impl Send for App {}
unsafe impl Sync for App {}

impl App {
    pub async fn new(settings: &AppSettings<'_>) -> Result<Arc<Self>> {
        let settings = Box::into_raw(Box::new(settings.into()));
        let (tx, rx) = oneshot::channel::<()>();
        let ptr = unsafe {
            create_app(
                settings,
                create_app_callback,
                Box::into_raw(Box::new(tx)) as *mut _,
            )
        };

        if ptr.is_null() {
            return Err(anyhow!("create app failed!"));
        }

        let this = Arc::new(Self {
            notify: Notify::new(),
            settings,
            ptr,
        });

        let this_ = this.clone();
        thread::spawn(move || {
            let args = args_ptr!();
            let ret = unsafe { app_run(this_.ptr, args.len() as c_int, args.as_ptr()) };
            if ret != 0 {
                panic!("app runing failed!, code: {}", ret)
            }

            this_.notify.notify_waiters();
        });

        rx.await?;
        Ok(this)
    }

    pub async fn create_browser<T>(
        &self,
        settings: &BrowserSettings<'_>,
        observer: T,
    ) -> Result<Arc<Browser>>
    where
        T: Observer + 'static,
    {
        Browser::new(self.ptr, settings, observer).await
    }

    pub async fn closed(&self) {
        self.notify.notified().await;
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe { app_exit(self.ptr) }
        drop(unsafe { Box::from_raw(self.settings) });
    }
}

extern "C" fn create_app_callback(ctx: *mut c_void) {
    let tx = unsafe { Box::from_raw(ctx as *mut Sender<()>) };
    tx.send(()).unwrap();
}
