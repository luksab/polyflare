pub mod lens_state;

pub mod scenes;

pub mod save_png;
pub mod state;
pub mod texture;

pub fn console_log(string: &str) {
    let array = js_sys::Array::new();
    array.push(&string.into());
    unsafe { web_sys::console::log(&array); }
}
