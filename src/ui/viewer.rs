use std::cell::RefCell;

use egui::{Color32, Pos2, Response, Style, Ui};

use super::{
    effect::{Effects, Forbidden},
    pin::{InPin, OutPin, PinInfo},
};

/// SnarlViewer is a trait for viewing a Snarl.
///
/// It can extract necessary data from the nodes and controls their
/// response to certain events.
pub trait SnarlViewer<T> {
    /// Returns title of the node.
    fn title(&mut self, node: &T) -> String;

    /// Renders the node's header.
    fn show_header(
        &mut self,
        idx: usize,
        node: &RefCell<T>,
        inputs: &[InPin<T>],
        outputs: &[OutPin<T>],
        ui: &mut Ui,
        scale: f32,
        effects: &mut Effects<T>,
    ) -> Response {
        let _ = (idx, node, inputs, outputs, scale, effects);
        ui.label(self.title(&*node.borrow()))
    }

    /// Returns number of output pins of the node.
    fn outputs(&mut self, node: &T) -> usize;

    /// Returns number of input pins of the node.
    fn inputs(&mut self, node: &T) -> usize;

    /// Renders the node's input pin.
    fn show_input(
        &mut self,
        pin: &InPin<T>,
        ui: &mut Ui,
        scale: f32,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<PinInfo>;

    /// Renders the node's output pin.
    fn show_output(
        &mut self,
        pin: &OutPin<T>,
        ui: &mut Ui,
        scale: f32,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<PinInfo>;

    /// Returns color of the node's input pin.
    /// Called when pin in not visible.
    fn input_color(&mut self, pin: &InPin<T>, style: &Style) -> Color32;

    /// Returns color of the node's output pin.
    /// Called when pin in not visible.
    fn output_color(&mut self, pin: &OutPin<T>, style: &Style) -> Color32;

    /// Show context menu for the snarl.
    ///
    /// This can be used to implement menu for adding new nodes.
    fn graph_menu(&mut self, pos: Pos2, ui: &mut Ui, scale: f32, effects: &mut Effects<T>) {
        let _ = (pos, ui, scale, effects);
    }

    /// Show context menu for the snarl.
    ///
    /// This can be used to implement menu for adding new nodes.
    fn node_menu(
        &mut self,
        idx: usize,
        node: &RefCell<T>,
        inputs: &[InPin<T>],
        outputs: &[OutPin<T>],
        ui: &mut Ui,
        scale: f32,
        effects: &mut Effects<T>,
    ) {
        let _ = (idx, node, inputs, outputs, ui, scale, effects);
    }

    /// Asks the viewer to connect two pins.
    ///
    /// This is usually happens when user drags a wire from one node's output pin to another node's input pin or vice versa.
    /// By default this method connects the pins and returns `Ok(())`.
    #[inline]
    fn connect(
        &mut self,
        from: &OutPin<T>,
        to: &InPin<T>,
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        effects.connect(from.id, to.id);
        Ok(())
    }

    /// Asks the viewer to disconnect two pins.
    #[inline]
    fn disconnect(
        &mut self,
        from: &OutPin<T>,
        to: &InPin<T>,
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        effects.disconnect(from.id, to.id);
        Ok(())
    }

    /// Asks the viewer to disconnect all wires from the output pin.
    ///
    /// This is usually happens when right-clicking on an output pin.
    /// By default this method disconnects the pins and returns `Ok(())`.
    #[inline]
    fn drop_outputs(&mut self, pin: &OutPin<T>, effects: &mut Effects<T>) -> Result<(), Forbidden> {
        effects.drop_outputs(pin.id);
        Ok(())
    }

    /// Asks the viewer to disconnect all wires from the input pin.
    ///
    /// This is usually happens when right-clicking on an input pin.
    /// By default this method disconnects the pins and returns `Ok(())`.
    #[inline]
    fn drop_inputs(&mut self, pin: &InPin<T>, effects: &mut Effects<T>) -> Result<(), Forbidden> {
        effects.drop_inputs(pin.id);
        Ok(())
    }
}
