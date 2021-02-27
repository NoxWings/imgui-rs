use std::borrow::Cow;
use std::ptr;
use std::thread;

use crate::context::Context;
use crate::string::ImStr;
use crate::sys;
use crate::Ui;

#[derive(Copy, Clone, Debug)]
enum Size {
    Vec(sys::ImVec2),
    Items {
        items_count: i32,
        height_in_items: i32,
    },
}
/// Builder for a list box widget
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct ListBox<'a> {
    label: &'a ImStr,
    size: Size,
}

impl<'a> ListBox<'a> {
    /// Constructs a new list box builder.
    pub const fn new(label: &'a ImStr) -> ListBox<'a> {
        ListBox {
            label,
            size: Size::Vec(sys::ImVec2::zero()),
        }
    }
    /// Sets the list box size based on the number of items that you want to make visible
    /// Size default to hold ~7.25 items.
    /// We add +25% worth of item height to allow the user to see at a glance if there are more items up/down, without looking at the scrollbar.
    /// We don't add this extra bit if items_count <= height_in_items. It is slightly dodgy, because it means a dynamic list of items will make the widget resize occasionally when it crosses that size.
    #[inline]
    pub const fn calculate_size(mut self, items_count: i32, height_in_items: i32) -> Self {
        self.size = Size::Items {
            items_count,
            height_in_items,
        };
        self
    }

    /// Sets the list box size based on the given width and height
    /// If width or height are 0 or smaller, a default value is calculated
    /// Helper to calculate the size of a listbox and display a label on the right.
    /// Tip: To have a list filling the entire window width, PushItemWidth(-1) and pass an non-visible label e.g. "##empty"
    ///
    /// Default: [0.0, 0.0], in which case the combobox calculates a sensible width and height
    #[inline]
    pub const fn size(mut self, size: [f32; 2]) -> Self {
        self.size = Size::Vec(sys::ImVec2::new(size[0], size[1]));
        self
    }
    /// Creates a list box and starts appending to it.
    ///
    /// Returns `Some(ListBoxToken)` if the list box is open. After content has been
    /// rendered, the token must be ended by calling `.end()`.
    ///
    /// Returns `None` if the list box is not open and no content should be rendered.
    #[must_use]
    pub fn begin(self, ui: &Ui) -> Option<ListBoxToken> {
        let should_render = unsafe {
            match self.size {
                Size::Vec(size) => sys::igBeginListBox(self.label.as_ptr(), size),
                Size::Items {
                    items_count,
                    height_in_items,
                } => {
                    let height_in_items_f = if height_in_items < 0 {
                        std::cmp::min(items_count, 7)
                    } else {
                        height_in_items
                    } as f32;
                    let style = *sys::igGetStyle();
                    let height = sys::igGetTextLineHeightWithSpacing() + height_in_items_f + style.FramePadding.y * 2.0;
                    let size = sys::ImVec2::new(0.0, height);
                    sys::igBeginListBox(self.label.as_ptr(), size)
                }
            }
        };
        if should_render {
            Some(ListBoxToken { ctx: ui.ctx })
        } else {
            None
        }
    }
    /// Creates a list box and runs a closure to construct the list contents.
    ///
    /// Note: the closure is not called if the list box is not open.
    pub fn build<F: FnOnce()>(self, ui: &Ui, f: F) {
        if let Some(list) = self.begin(ui) {
            f();
            list.end(ui);
        }
    }
}

/// Tracks a list box that must be ended by calling `.end()`
#[must_use]
pub struct ListBoxToken {
    ctx: *const Context,
}

impl ListBoxToken {
    /// Ends a list box
    pub fn end(mut self, _: &Ui) {
        self.ctx = ptr::null();
        unsafe { sys::igEndListBox() };
    }
}

impl Drop for ListBoxToken {
    fn drop(&mut self) {
        if !self.ctx.is_null() && !thread::panicking() {
            panic!("A ListBoxToken was leaked. Did you call .end()?");
        }
    }
}

/// # Convenience functions
impl<'a> ListBox<'a> {
    /// Builds a simple list box for choosing from a slice of values
    pub fn build_simple<T, L>(
        self,
        ui: &Ui,
        current_item: &mut usize,
        items: &[T],
        label_fn: &L,
    ) -> bool
    where
        for<'b> L: Fn(&'b T) -> Cow<'b, ImStr>,
    {
        use crate::widget::selectable::Selectable;
        let mut result = false;
        let lb = self;
        if let Some(_cb) = lb.begin(ui) {
            for (idx, item) in items.iter().enumerate() {
                let text = label_fn(item);
                let selected = idx == *current_item;
                if Selectable::new(&text).selected(selected).build(ui) {
                    *current_item = idx;
                    result = true;
                }
            }
            _cb.end(ui);
        }
        result
    }
}
