use flume::Sender;
use gtk::gdk_pixbuf::InterpType;
use gtk::prelude::ImageExt;
use gtk::Image;
use gtk::TreePath;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiEvents;
use std::time::Duration;
use ui_gtk::iconloader::IconLoader;

const EV_QUEUE_SEND_DURATION: Duration = Duration::from_millis(100);

#[allow(dead_code)]
pub const MOUSE_BUTTON_MID: u32 = 2;
#[allow(dead_code)]
pub const MOUSE_BUTTON_RIGHT: u32 = 3;
pub const MOUSE_BUTTON_LEFT: u32 = 1;

// pub const ICON_SIZE : i32 = 24;
pub const DIALOG_ICON_SIZE: i32 = 24;

pub struct EvSenderWrapper(pub Sender<GuiEvents>);
impl EvSenderWrapper {
    pub fn sendw(&self, ev: GuiEvents) {
        if let Err(e) = self.0.send_timeout(ev.clone(), EV_QUEUE_SEND_DURATION) {
            error!("g_o_t: skipped event {:?}   {:?}", &ev, &e);
        }
    }
}

pub struct EvSenderCache(pub Sender<GuiEvents>, pub GuiEvents);
impl EvSenderCache {
    pub fn send(&self) {
        if let Err(e) = self.0.send_timeout(self.1.clone(), EV_QUEUE_SEND_DURATION) {
            error!("g_o_t: skipped event {:?}   {:?}", &self.1, &e);
        }
    }
}

#[derive(Default, Debug)]
pub struct DragState {
    pub inserted: Option<Vec<u16>>, //  ID not delivered on drag
    pub deleted: Option<Vec<u16>>,
    pub drag_start_path: Option<TreePath>,
    // pub in_store_update: bool,
}

impl DragState {
    pub fn block_row_activated(&self) -> bool {
        self.drag_start_path.is_some()  // || self.in_store_update
    }
}

pub fn process_icon_to_image(
    input: Option<&AValue>,
    dest_image: &Image,
    err_help: &String,
) -> bool {
    if input.is_none() {
        warn!("no input value for icon: {} ", err_help);
        return false;
    }
    let o_str = input.as_ref().unwrap().str();
    if o_str.is_none() {
        warn!("no input string for icon: {} ", err_help);
        return false;
    }
    let icon_str = o_str.unwrap();
    process_string_to_image(&icon_str, dest_image, err_help, DIALOG_ICON_SIZE)
}

pub fn process_string_to_image(
    icon_str: &str,
    dest_image: &Image,
    err_help: &String,
    size: i32,
) -> bool {
    let buf = IconLoader::decompress_string_to_vec(icon_str);
    if buf.is_empty() {
        warn!("empty icon_buffer: {} ", err_help);
        return false;
    }
    let r_pb = IconLoader::vec_to_pixbuf(&buf);
    if r_pb.is_err() {
        warn!(
            "cannot create pixbuf {} {:?} #buf={}",
            err_help,
            &r_pb.err(),
            buf.len()
        );
        return false;
    }
    let pb = r_pb.unwrap();

    let r_pbscaled = pb.scale_simple(size, size, InterpType::Bilinear);
    if r_pbscaled.is_none() {
        warn!("cannot rescale pixbuf {}  ", err_help);
        return false;
    }
    let pb_scaled = r_pbscaled.unwrap();
    dest_image.set_pixbuf(Some(&pb_scaled));
    true
}

pub fn dd_get_uint(dialogdata: &[AValue], index: usize, default: u32) -> u32 {
    if let Some(av) = dialogdata.get(index) {
        if let Some(i) = av.uint() {
            return i;
        } else {
            error!("dd value {} missing: {:?}", index, av);
        }
    } else {
        error!("dd index {} missing", index);
    }
    default
}
