use egui::{Modifiers, PointerButton};

/// Struct holding keyboard modifiers and mouse button.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModifierClick {
    /// Keyboard modifiers for this action.
    pub modifiers: Modifiers,

    /// Mouse buttons for this action.
    pub mouse_button: PointerButton,
}

/// Config options for Snarl.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SnarlConfig {
    /// Controls key bindings.

    /// Action used to draw selection rect.
    /// Defaults to [`PointerButton::Primary`] && `[Modifiers::SHIFT].
    pub rect_select: ModifierClick,

    /// Action used to remove hovered wire.
    /// Defaults to [`PointerButton::Secondary`].
    pub remove_hovered_wire: ModifierClick,

    /// Action used to deselect all nodes.
    /// Defaults to [`PointerButton::Primary`].
    pub deselect_all_nodes: ModifierClick,

    /// Action used to cancel wire drag.
    /// Defaults to [`PointerButton::Secondary`].
    pub cancel_wire_drag: ModifierClick,

    /// Action used to click on pin.
    /// Defaults to [`PointerButton::Secondary`].
    pub click_pin: ModifierClick,

    /// Action used to drag pin.
    /// Defaults to [`PointerButton::Primary`] && [`Modifiers::COMMAND`].
    pub drag_pin: ModifierClick,

    /// Action used to avoid popup menu on wire drop.
    /// Defaults to [`PointerButton::Primary`] && [`Modifiers::SHIFT`].
    pub no_menu: ModifierClick,

    /// Action used to click node.
    /// Defaults to [`PointerButton::Primary`].
    pub click_node: ModifierClick,

    /// Action used to drag node.
    /// Defaults to [`PointerButton::Primary`].
    pub drag_node: ModifierClick,

    /// Action used to select node.
    /// Defaults to [`PointerButton::Primary`] && [`Modifiers::SHIFT`].
    pub select_node: ModifierClick,

    /// Action used to deselect node.
    /// Defaults to [`PointerButton::Primary`] && [`Modifiers::COMMAND`].
    pub deselect_node: ModifierClick,

    /// Action used to click node header.
    /// Defaults to [`PointerButton::Primary`]``.
    pub click_header: ModifierClick,

    #[doc(hidden)]
    #[cfg_attr(feature = "serde", serde(skip_serializing, default))]
    /// Do not access other than with .., here to emulate `#[non_exhaustive(pub)]`
    pub _non_exhaustive: (),
}

impl SnarlConfig {
    /// Creates new [`SnarlConfig`] filled with default values.
    #[must_use]
    pub const fn new() -> Self {
        SnarlConfig {
            rect_select: ModifierClick {
                modifiers: Modifiers::SHIFT,
                mouse_button: PointerButton::Primary,
            },
            remove_hovered_wire: ModifierClick {
                modifiers: Modifiers::NONE,
                mouse_button: PointerButton::Secondary,
            },
            deselect_all_nodes: ModifierClick {
                modifiers: Modifiers::COMMAND,
                mouse_button: PointerButton::Primary,
            },
            cancel_wire_drag: ModifierClick {
                modifiers: Modifiers::NONE,
                mouse_button: PointerButton::Secondary,
            },
            click_pin: ModifierClick {
                modifiers: Modifiers::NONE,
                mouse_button: PointerButton::Secondary,
            },
            drag_pin: ModifierClick {
                modifiers: Modifiers::COMMAND,
                mouse_button: PointerButton::Primary,
            },
            no_menu: ModifierClick {
                modifiers: Modifiers::SHIFT,
                mouse_button: PointerButton::Primary,
            },
            click_node: ModifierClick {
                modifiers: Modifiers::NONE,
                mouse_button: PointerButton::Primary,
            },
            drag_node: ModifierClick {
                modifiers: Modifiers::NONE,
                mouse_button: PointerButton::Primary,
            },
            select_node: ModifierClick {
                modifiers: Modifiers::SHIFT,
                mouse_button: PointerButton::Primary,
            },
            deselect_node: ModifierClick {
                modifiers: Modifiers::COMMAND,
                mouse_button: PointerButton::Primary,
            },
            click_header: ModifierClick {
                modifiers: Modifiers::NONE,
                mouse_button: PointerButton::Primary,
            },

            _non_exhaustive: (),
        }
    }
}
