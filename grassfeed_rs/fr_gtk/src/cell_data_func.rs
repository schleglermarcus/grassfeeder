/**
https://gtk-rs.org/gtk-rs-core/stable/0.15/docs/pango/struct.AttrSize.html
https://gtk-rs.org/gtk-rs-core/stable/0.14/docs/pango_sys/struct.PangoAttrSize.html
 */
use gtk::pango::AttrInt;
use gtk::pango::AttrList;
use gtk::pango::Attribute;
use gtk::pango::Weight;
use gtk::prelude::Cast;
use gtk::prelude::CellRendererTextExt;
use gtk::prelude::TreeModelExt;
use gtk::CellRenderer;
use gtk::CellRendererText;
use gtk::TreeIter;
use gtk::TreeModel;
use gtk::TreeViewColumn;
use gui_layer::gui_values::FontAttributes;
use std::marker::PhantomData;

pub trait BoldFuncDiscriminator {
    fn column_nr() -> i32;

    fn attrlist(act_bits: u32) -> AttrList {
        let (fontsize, is_read, is_folder, _is_transparent) =
            FontAttributes::from_activation_bits(act_bits);
        let r = AttrList::new();
        if !is_read && !is_folder {
            add_font_weight(&r);
        }
        add_font_size(&r, fontsize);
        r
    }
}

#[cfg(not(feature = "legacy3gtk14"))]
pub fn add_font_weight(r: &AttrList) {
    r.insert(Attribute::from(AttrInt::new_weight(Weight::Bold)));
}

#[cfg(feature = "legacy3gtk14")]
pub fn add_font_weight(r: &AttrList) {
    r.insert(gtk::pango::Attribute::new_weight(Weight::Bold));
}

#[cfg(not(feature = "legacy3gtk14"))]
pub fn add_font_size(r: &AttrList, fontsize: u32) {
    if fontsize > 0 {
        r.insert(Attribute::from(gtk::pango::AttrSize::new(
            fontsize as i32 * gtk::pango::SCALE,
        )));
    }
}

#[cfg(feature = "legacy3gtk14")]
pub fn add_font_size(r: &AttrList, fontsize: u32) {
    // do nothing. Downgrade of functionality with gtk-0.14.   Later   add newer gtk packages
}

#[derive(Default)]
pub struct TreeBoldDiscr {}

impl BoldFuncDiscriminator for TreeBoldDiscr {
    fn column_nr() -> i32 {
        6
    }
}

#[derive(Default)]
pub struct ListBoldDiscr {}

impl BoldFuncDiscriminator for ListBoldDiscr {
    fn column_nr() -> i32 {
        4
    }
}

#[derive(Default)]
pub struct BoldFunction<D>
where
    D: BoldFuncDiscriminator,
{
    _p: PhantomData<D>,
}

impl<D> BoldFunction<D>
where
    D: BoldFuncDiscriminator,
{
    pub fn tree_switch_bold(
        _t_v_col: &TreeViewColumn,
        ce_re: &CellRenderer,
        t_model: &TreeModel,
        t_iter: &TreeIter,
    ) {
        if let Some(crt) = (*ce_re).downcast_ref::<CellRendererText>() {
            crt.set_attributes(None);
            let val: gtk::glib::Value = (*t_model).value(t_iter, D::column_nr());
            if let Ok(col_val) = val.get::<u32>() {
                crt.set_attributes(Some(&D::attrlist(col_val))); // , t_v_col.sort_column_id()
            }
        }
    }
}

//
