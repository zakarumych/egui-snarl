use egui::{Painter, Pos2, Rect, Style, Ui, emath::TSTransform};

use crate::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};

use super::{
    BackgroundPattern, NodeLayout, SnarlStyle,
    pin::{AnyPins, SnarlPin},
};

/// `SnarlViewer` is a trait for viewing a Snarl.
///
/// It can extract necessary data from the nodes and controls their
/// response to certain events.
pub trait SnarlViewer<T> {
    /// Returns title of the node.
    fn title(&mut self, node: &T) -> String;

    /// Returns the node's frame.
    /// All node's elements will be rendered inside this frame.
    /// Except for pins if they are configured to be rendered outside of the frame.
    ///
    /// Returns `default` by default.
    /// `default` frame is taken from the [`SnarlStyle::node_frame`] or constructed if it's `None`.
    ///
    /// Override this method to customize the frame for specific nodes.
    fn node_frame(
        &mut self,
        default: egui::Frame,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        snarl: &Snarl<T>,
    ) -> egui::Frame {
        let _ = (node, inputs, outputs, snarl);
        default
    }

    /// Returns the node's header frame.
    ///
    /// This frame would be placed on top of the node's frame.
    /// And header UI (see [`show_header`]) will be placed inside this frame.
    ///
    /// Returns `default` by default.
    /// `default` frame is taken from the [`SnarlStyle::header_frame`],
    /// or [`SnarlStyle::node_frame`] with removed shadow if `None`,
    /// or constructed if both are `None`.
    fn header_frame(
        &mut self,
        default: egui::Frame,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        snarl: &Snarl<T>,
    ) -> egui::Frame {
        let _ = (node, inputs, outputs, snarl);
        default
    }
    /// Checks if node has a custom egui style.
    #[inline]
    fn has_node_style(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        snarl: &Snarl<T>,
    ) -> bool {
        let _ = (node, inputs, outputs, snarl);
        false
    }

    /// Modifies the node's egui style
    fn apply_node_style(
        &mut self,
        style: &mut Style,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        snarl: &Snarl<T>,
    ) {
        let _ = (style, node, inputs, outputs, snarl);
    }

    /// Returns elements layout for the node.
    ///
    /// Node consists of 5 parts: header, body, footer, input pins and output pins.
    /// See [`NodeLayout`] for available placements.
    ///
    /// Returns `default` by default.
    /// `default` layout is taken from the [`SnarlStyle::node_layout`] or constructed if it's `None`.
    /// Override this method to customize the layout for specific nodes.
    #[inline]
    fn node_layout(
        &mut self,
        default: NodeLayout,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        snarl: &Snarl<T>,
    ) -> NodeLayout {
        let _ = (node, inputs, outputs, snarl);
        default
    }

    /// Renders elements inside the node's header frame.
    ///
    /// This is the good place to show the node's title and controls related to the whole node.
    ///
    /// By default it shows the node's title.
    #[inline]
    fn show_header(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (inputs, outputs);
        ui.label(self.title(&snarl[node]));
    }

    /// Returns number of input pins of the node.
    ///
    /// [`SnarlViewer::show_input`] will be called for each input in range `0..inputs()`.
    fn inputs(&mut self, node: &T) -> usize;

    /// Renders one specified node's input element and returns drawer for the corresponding pin.
    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut Ui,
        snarl: &mut Snarl<T>,
    ) -> impl SnarlPin + 'static;

    /// Returns number of output pins of the node.
    ///
    /// [`SnarlViewer::show_output`] will be called for each output in range `0..outputs()`.
    fn outputs(&mut self, node: &T) -> usize;

    /// Renders the node's output.
    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        snarl: &mut Snarl<T>,
    ) -> impl SnarlPin + 'static;

    /// Checks if node has something to show in body - between input and output pins.
    #[inline]
    fn has_body(&mut self, node: &T) -> bool {
        let _ = node;
        false
    }

    /// Renders the node's body.
    #[inline]
    fn show_body(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, snarl);
    }

    /// Checks if node has something to show in footer - below pins and body.
    #[inline]
    fn has_footer(&mut self, node: &T) -> bool {
        let _ = node;
        false
    }

    /// Renders the node's footer.
    #[inline]
    fn show_footer(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, snarl);
    }

    /// Reports the final node's rect after rendering.
    ///
    /// It aimed to be used for custom positioning of nodes that requires node dimensions for calculations.
    /// Node's position can be modified directly in this method.
    #[inline]
    fn final_node_rect(&mut self, node: NodeId, rect: Rect, ui: &mut Ui, snarl: &mut Snarl<T>) {
        let _ = (node, rect, ui, snarl);
    }

    /// Checks if node has something to show in on-hover popup.
    #[inline]
    fn has_on_hover_popup(&mut self, node: &T) -> bool {
        let _ = node;
        false
    }

    /// Renders the node's on-hover popup.
    #[inline]
    fn show_on_hover_popup(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, snarl);
    }

    /// Checks if wire has something to show in widget.
    /// This may not be called if wire is invisible.
    #[inline]
    fn has_wire_widget(&mut self, from: &OutPinId, to: &InPinId, snarl: &Snarl<T>) -> bool {
        let _ = (from, to, snarl);
        false
    }

    /// Renders the wire's widget.
    /// This may not be called if wire is invisible.
    #[inline]
    fn show_wire_widget(&mut self, from: &OutPin, to: &InPin, ui: &mut Ui, snarl: &mut Snarl<T>) {
        let _ = (from, to, ui, snarl);
    }

    /// Checks if the snarl has something to show in context menu if right-clicked or long-touched on empty space at `pos`.
    #[inline]
    fn has_graph_menu(&mut self, pos: Pos2, snarl: &mut Snarl<T>) -> bool {
        let _ = (pos, snarl);
        false
    }

    /// Show context menu for the snarl.
    ///
    /// This can be used to implement menu for adding new nodes.
    #[inline]
    fn show_graph_menu(&mut self, pos: Pos2, ui: &mut Ui, snarl: &mut Snarl<T>) {
        let _ = (pos, ui, snarl);
    }

    /// Checks if the snarl has something to show in context menu if wire drag is stopped at `pos`.
    #[inline]
    fn has_dropped_wire_menu(&mut self, src_pins: AnyPins, snarl: &mut Snarl<T>) -> bool {
        let _ = (src_pins, snarl);
        false
    }

    /// Show context menu for the snarl. This menu is opened when releasing a pin to empty
    /// space. It can be used to implement menu for adding new node, and directly
    /// connecting it to the released wire.
    #[inline]
    fn show_dropped_wire_menu(
        &mut self,
        pos: Pos2,
        ui: &mut Ui,
        src_pins: AnyPins,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (pos, ui, src_pins, snarl);
    }

    /// Checks if the node has something to show in context menu if right-clicked or long-touched on the node.
    #[inline]
    fn has_node_menu(&mut self, node: &T) -> bool {
        let _ = node;
        false
    }

    /// Show context menu for the snarl.
    ///
    /// This can be used to implement menu for adding new nodes.
    #[inline]
    fn show_node_menu(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, snarl);
    }

    /// Asks the viewer to connect two pins.
    ///
    /// This is usually happens when user drags a wire from one node's output pin to another node's input pin or vice versa.
    /// By default this method connects the pins and returns `Ok(())`.
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<T>) {
        snarl.connect(from.id, to.id);
    }

    /// Asks the viewer to disconnect two pins.
    #[inline]
    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<T>) {
        snarl.disconnect(from.id, to.id);
    }

    /// Asks the viewer to disconnect all wires from the output pin.
    ///
    /// This is usually happens when right-clicking on an output pin.
    /// By default this method disconnects the pins and returns `Ok(())`.
    #[inline]
    fn drop_outputs(&mut self, pin: &OutPin, snarl: &mut Snarl<T>) {
        snarl.drop_outputs(pin.id);
    }

    /// Asks the viewer to disconnect all wires from the input pin.
    ///
    /// This is usually happens when right-clicking on an input pin.
    /// By default this method disconnects the pins and returns `Ok(())`.
    #[inline]
    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<T>) {
        snarl.drop_inputs(pin.id);
    }

    /// Draws background of the snarl view.
    ///
    /// By default it draws the background pattern using [`BackgroundPattern::draw`].
    ///
    /// If you want to draw the background yourself, you can override this method.
    #[inline]
    fn draw_background(
        &mut self,
        background: Option<&BackgroundPattern>,
        viewport: &Rect,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
        snarl: &Snarl<T>,
    ) {
        let _ = snarl;

        if let Some(background) = background {
            background.draw(viewport, snarl_style, style, painter);
        }
    }

    /// Informs the viewer what is the current transform of the snarl view
    /// and allows viewer to override it.
    ///
    /// This method is called in the beginning of the graph rendering.
    ///
    /// By default it does nothing.
    #[inline]
    fn current_transform(&mut self, to_global: &mut TSTransform, snarl: &mut Snarl<T>) {
        let _ = (to_global, snarl);
    }
}
