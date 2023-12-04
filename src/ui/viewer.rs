use std::cell::RefCell;

use egui::{Color32, Pos2, Response, Ui};

use super::{
    effect::{Effects, Forbidden},
    pin::{InPin, OutPin, PinInfo},
};

/// SnarlViewer is a trait for viewing a Snarl.
///
/// It can extract necessary data from the nodes and controls their
/// response to certain events.
pub trait SnarlViewer<T> {
    /// Called to create new node in the Snarl.
    ///
    /// Returns response with effects to be applied to the Snarl after the node is added.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` error if the node cannot be added.
    #[inline]
    fn add_node(
        &mut self,
        idx: usize,
        node: &T,
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        let _ = (idx, node, effects);
        Ok(())
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

    /// Called when a node is about to be removed.
    ///
    /// # Arguments
    ///
    /// * `node` - Node that is about to be removed.
    /// * `inputs` - Array of input pins connected to the node.
    /// * `outputs` - Array of output pins connected to the node.
    ///
    /// Returns response with effects to be applied to the Snarl after the node is removed.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` error if the node cannot be removed.
    #[inline]
    fn remove_node(
        &mut self,
        idx: usize,
        node: &RefCell<T>,
        inputs: &[InPin<T>],
        outputs: &[OutPin<T>],
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        let _ = (idx, node, inputs, outputs);
        effects.remove_node(idx);
        Ok(())
    }

    fn node_picker(&mut self, ui: &mut Ui) -> egui::InnerResponse<Option<T>>;

    fn title<'a>(&'a mut self, node: &'a T) -> &'a str;

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

    fn outputs(&mut self, node: &T) -> usize;

    fn inputs(&mut self, node: &T) -> usize;

    fn show_input(
        &mut self,
        pin: &InPin<T>,
        ui: &mut Ui,
        scale: f32,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<PinInfo>;

    fn show_output(
        &mut self,
        pin: &OutPin<T>,
        ui: &mut Ui,
        scale: f32,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<PinInfo>;

    fn input_color(&mut self, pin: &InPin<T>) -> Color32;
    fn output_color(&mut self, pin: &OutPin<T>) -> Color32;

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
}
