use std::ffi::{c_char, c_int};

use crate::ptr::AsCStr;

use super::RawBrowser;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtons {
    Left,
    Right,
    Middle,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifiers {
    None = 0,
    Shift = 1,
    Ctrl = 2,
    Alt = 3,
    Win = 4,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchEventType {
    TouchReleased = 0,
    TouchPressed = 1,
    TouchMoved = 2,
    TouchCancelled = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPointerType {
    Touch = 0,
    Mouse = 1,
    Pen = 2,
    Eraser = 3,
    Unknown = 4,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

extern "C" {
    fn browser_send_mouse_click(browser: *const RawBrowser, button: MouseButtons, pressed: bool);
    fn browser_send_mouse_click_with_pos(
        browser: *const RawBrowser,
        button: MouseButtons,
        pressed: bool,
        x: c_int,
        y: c_int,
    );
    fn browser_send_mouse_wheel(browser: *const RawBrowser, x: c_int, y: c_int);
    fn browser_send_mouse_move(browser: *const RawBrowser, x: c_int, y: c_int);
    fn browser_send_keyboard(
        browser: *const RawBrowser,
        scan_code: c_int,
        pressed: bool,
        modifiers: Modifiers,
    );
    fn browser_send_touch(
        browser: *const RawBrowser,
        id: c_int,
        x: c_int,
        y: c_int,
        kind: TouchEventType,
        pointer_type: TouchPointerType,
    );
    fn browser_send_ime_composition(browser: *const RawBrowser, input: *const c_char);
    fn browser_send_ime_set_composition(
        browser: *const RawBrowser,
        input: *const c_char,
        x: c_int,
        y: c_int,
    );
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ActionState {
    Down,
    Up,
}

impl ActionState {
    fn is_pressed(self) -> bool {
        self == Self::Down
    }
}

#[derive(Debug, Clone)]
pub enum MouseAction {
    Click(MouseButtons, ActionState, Option<Position>),
    Move(Position),
    Wheel(Position),
}

#[derive(Debug)]
pub enum ImeAction<'a> {
    Composition(&'a str),
    Pre(&'a str, i32, i32),
}

pub(crate) struct Control;

impl Control {
    pub fn on_mouse(ptr: *const RawBrowser, action: MouseAction) {
        match action {
            MouseAction::Move(pos) => unsafe { browser_send_mouse_move(ptr, pos.x, pos.y) },
            MouseAction::Wheel(pos) => unsafe { browser_send_mouse_wheel(ptr, pos.x, pos.y) },
            MouseAction::Click(button, state, pos) => {
                if let Some(pos) = pos {
                    unsafe {
                        browser_send_mouse_click_with_pos(
                            ptr,
                            button,
                            state.is_pressed(),
                            pos.x,
                            pos.y,
                        )
                    }
                } else {
                    unsafe { browser_send_mouse_click(ptr, button, state.is_pressed()) }
                }
            }
        }
    }

    pub fn on_keyboard(
        ptr: *const RawBrowser,
        scan_code: u32,
        state: ActionState,
        modifiers: Modifiers,
    ) {
        unsafe { browser_send_keyboard(ptr, scan_code as c_int, state.is_pressed(), modifiers) }
    }

    pub fn on_touch(
        ptr: *const RawBrowser,
        id: i32,
        x: i32,
        y: i32,
        ty: TouchEventType,
        pointer_type: TouchPointerType,
    ) {
        unsafe { browser_send_touch(ptr, id, x, y, ty, pointer_type) }
    }

    pub fn on_ime(ptr: *const RawBrowser, action: ImeAction) {
        match action {
            ImeAction::Composition(input) => unsafe {
                browser_send_ime_composition(ptr, input.as_c_str().ptr)
            },
            ImeAction::Pre(input, x, y) => unsafe {
                browser_send_ime_set_composition(ptr, input.as_c_str().ptr, x, y)
            },
        }
    }
}
