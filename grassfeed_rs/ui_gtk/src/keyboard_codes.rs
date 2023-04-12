use gdk_sys;
use gui_layer::gui_values::KeyCodes;

// https://gtk-rs.org/gtk3-rs/stable/latest/docs/gdk_sys/index.html
#[allow(unused_variables)]
pub fn from_gdk_sys(code: isize) -> KeyCodes {
    match code as i32 {
        gdk_sys::GDK_KEY_Tab => KeyCodes::Tab,
        gdk_sys::GDK_KEY_ISO_Left_Tab => KeyCodes::ShiftTab,
        gdk_sys::GDK_KEY_KP_Tab => KeyCodes::Tab,
        gdk_sys::GDK_KEY_space => KeyCodes::Space,
        gdk_sys::GDK_KEY_Escape => KeyCodes::Escape,
        gdk_sys::GDK_KEY_KP_Enter | gdk_sys::GDK_KEY_ISO_Enter | gdk_sys::GDK_KEY_Return => {
            KeyCodes::Enter
        }
        gdk_sys::GDK_KEY_F1 => KeyCodes::F1,
        gdk_sys::GDK_KEY_F2 => KeyCodes::F2,
        gdk_sys::GDK_KEY_F3 => KeyCodes::F3,
        gdk_sys::GDK_KEY_F4 => KeyCodes::F4,
        gdk_sys::GDK_KEY_Up => KeyCodes::CursorUp,
        gdk_sys::GDK_KEY_Down => KeyCodes::CursorDown,
        gdk_sys::GDK_KEY_Right => KeyCodes::CursorRight,
        gdk_sys::GDK_KEY_Left => KeyCodes::CursorLeft,
        gdk_sys::GDK_KEY_Delete => KeyCodes::Delete,

        gdk_sys::GDK_KEY_A => KeyCodes::Key_A,
        gdk_sys::GDK_KEY_a => KeyCodes::Key_a,
        gdk_sys::GDK_KEY_B => KeyCodes::Key_B,
        gdk_sys::GDK_KEY_b => KeyCodes::Key_b,
        gdk_sys::GDK_KEY_N => KeyCodes::Key_N,
        gdk_sys::GDK_KEY_n => KeyCodes::Key_n,
        gdk_sys::GDK_KEY_s => KeyCodes::Key_s,
        gdk_sys::GDK_KEY_v => KeyCodes::Key_v,
        gdk_sys::GDK_KEY_x => KeyCodes::Key_x,
        _ => KeyCodes::Nothing,
    }
}
