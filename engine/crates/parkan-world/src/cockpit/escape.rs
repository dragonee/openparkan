//! What Esc's key going down puts away before its binding, `CMD_ROLLBACK_STATE`, sees it.
//!
//! A key-down reaches the game's input listeners first (`iron3d.dll:0x100a0eb8`), in every
//! view state but 0, and the first to answer 1 keeps it from the bindings (`0x100a0fe5`). The
//! game's own listener's key-down handler (`0x10070db0`, slot 0 of `0x100e6490`) answers Esc
//! for the first layer up, in this order. See `docs/40-command-mode.md`, "Input".

/// A layer Esc puts away, one a press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// The objectives screen closes (`0x10070e85`).
    Objectives,
    /// A building being placed is cancelled, as the right button cancels it (`0x10070ed1`).
    Placement,
    /// The message box is hidden, as `CMD_PAGER` hides it (`0x10070eef`, 729).
    MessageBox,
    /// In the commander's view, the open satellite map closes (`0x10071027`).
    Map,
    /// In the commander's view, the panel turns from its page to page 0 (`0x1007104b`).
    Page,
}

/// What is up as Esc goes down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Up {
    pub objectives: bool,
    pub placing: bool,
    pub message_box: bool,
    /// View state 2, the commander's view.
    pub commander_view: bool,
    pub map: bool,
    /// The panel's page is not 0.
    pub page: bool,
}

/// The layer Esc takes, or none, when the binding has it and rolls the mode back.
pub fn peel(up: Up) -> Option<Layer> {
    if up.objectives {
        Some(Layer::Objectives)
    } else if up.placing {
        Some(Layer::Placement)
    } else if up.message_box {
        Some(Layer::MessageBox)
    } else if up.commander_view && up.map {
        Some(Layer::Map)
    } else if up.commander_view && up.page {
        Some(Layer::Page)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_peels_the_objectives_a_placement_the_message_box_the_map_and_the_page_before_leaving() {
        let mut up = Up {
            objectives: true,
            placing: true,
            message_box: true,
            commander_view: true,
            map: true,
            page: true,
        };
        let mut peeled = Vec::new();
        while let Some(layer) = peel(up) {
            peeled.push(layer);
            match layer {
                Layer::Objectives => up.objectives = false,
                Layer::Placement => up.placing = false,
                Layer::MessageBox => up.message_box = false,
                Layer::Map => up.map = false,
                Layer::Page => up.page = false,
            }
        }
        assert_eq!(peeled, [Layer::Objectives, Layer::Placement, Layer::MessageBox, Layer::Map, Layer::Page]);
    }

    #[test]
    fn the_map_and_the_page_are_the_commanders_view_alone() {
        let up = Up { map: true, page: true, ..Up::default() };
        assert_eq!(peel(up), None, "in the cockpit the binding has Esc");
        assert_eq!(peel(Up { message_box: true, ..up }), Some(Layer::MessageBox), "in any view");
    }
}
