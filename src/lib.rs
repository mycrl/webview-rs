mod app;
mod browser;
mod ptr;

use std::{
    env::args,
    ffi::{c_char, c_int},
};

use ptr::AsCStr;

pub use app::{App, AppSettings};
pub use browser::{
    bridge::BridgeObserver,
    control::{
        ActionState, ImeAction, Modifiers, MouseAction, MouseButtons, Position, Rect,
        TouchEventType, TouchPointerType,
    },
    Browser, BrowserSettings, BrowserState, Observer, HWND,
};

extern "C" {
    fn execute_sub_process(argc: c_int, argv: *const *const c_char);
}

#[macro_export]
macro_rules! args_ptr {
    () => {
        std::env::args()
            .map(|arg| arg.as_c_str())
            .collect::<Vec<_>>()
            .iter()
            .map(|arg| arg.ptr)
            .collect::<Vec<*const c_char>>()
    };
}

pub fn execute_subprocess() -> ! {
    if tokio::runtime::Handle::try_current().is_ok() {
        panic!("webview sub process does not work in tokio runtime!");
    }

    let args = args_ptr!();
    unsafe { execute_sub_process(args.len() as c_int, args.as_ptr()) };
    unreachable!("sub process closed, this is a bug!")
}

pub fn is_subprocess() -> bool {
    args().find(|v| v.contains("--type")).is_some()
}
