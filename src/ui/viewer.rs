use egui::{Color32, Painter, Pos2, Rect, Style, Ui};

use crate::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};

use super::{pin::AnyPins, BackgroundPattern, NodeLayout, PinInfo, SnarlStyle, Viewport};

/// SnarlViewer is a trait for viewing a Snarl.
///
/// It can extract necessary data from the nodes and controls their
/// response to certain events.
pub trait SnarlViewer<T> {
    /// Returns title of the node.
    fn title(&mut self, node: &T) -> String;

    /// Returns the node's frame.
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

    /// Returns layout override for the node.
    ///
    /// This method can be used to override the default layout of the node.
    /// By default it returns `None` and layout from the style is used.
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

    /// Renders the node's header.
    #[inline]
    fn show_header(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (inputs, outputs, scale);
        ui.label(self.title(&snarl[node]));
    }

    /// Returns number of input pins of the node.
    ///
    /// [`SnarlViewer::show_input`] and [`SnarlViewer::draw_input_pin`] will be called for each input in range `0..inputs()`.
    fn inputs(&mut self, node: &T) -> usize;

    /// Renders the node's input.
    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, scale: f32, snarl: &mut Snarl<T>)
        -> PinInfo;

    /// Returns number of output pins of the node.
    ///
    /// [`SnarlViewer::show_output`] and [`SnarlViewer::show_output_ping`] will be called for each output in range `0..outputs()`.
    fn outputs(&mut self, node: &T) -> usize;

    /// Renders the node's output.
    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<T>,
    ) -> PinInfo;

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
        scale: f32,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, scale, snarl);
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
        scale: f32,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, scale, snarl);
    }

    /// Reports the final node's rect after rendering.
    ///
    /// It aimed to be used for custom positioning of nodes that requires node dimensions for calculations.
    /// Node's position can be modified directly in this method.
    #[inline]
    fn final_node_rect(
        &mut self,
        node: NodeId,
        ui_rect: Rect,
        graph_rect: Rect,
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, ui_rect, graph_rect, ui, scale, snarl);
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
        scale: f32,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, scale, snarl);
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
    fn show_wire_widget(
        &mut self,
        from: &OutPin,
        to: &InPin,
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (from, to, ui, scale, snarl);
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
    fn show_graph_menu(&mut self, pos: Pos2, ui: &mut Ui, scale: f32, snarl: &mut Snarl<T>) {
        let _ = (pos, ui, scale, snarl);
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
        scale: f32,
        src_pins: AnyPins,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (pos, ui, scale, src_pins, snarl);
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
        scale: f32,
        snarl: &mut Snarl<T>,
    ) {
        let _ = (node, inputs, outputs, ui, scale, snarl);
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

    /// Draws the node's input pin.
    ///
    /// This method is called after [`SnarlViewer::show_input`] and can be used to draw the pin shape.
    /// By default it draws a pin with the shape and style returned by [`SnarlViewer::show_input`].
    ///
    /// If you want to draw the pin yourself, you can override this method.
    #[allow(clippy::too_many_arguments)]
    fn draw_input_pin(
        &mut self,
        pin: &InPin,
        pin_info: &PinInfo,
        pos: Pos2,
        size: f32,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
        scale: f32,
        snarl: &Snarl<T>,
    ) -> Color32 {
        let _ = (pin, snarl);

        pin_info.draw(pos, size, snarl_style, style, painter, scale)
    }

    /// Draws the node's output pin.
    ///
    /// This method is called after [`SnarlViewer::show_output`] and can be used to draw the pin shape.
    /// By default it draws a pin with the shape and style returned by [`SnarlViewer::show_output`].
    ///
    /// If you want to draw the pin yourself, you can override this method.
    #[allow(clippy::too_many_arguments)]
    fn draw_output_pin(
        &mut self,
        pin: &OutPin,
        pin_info: &PinInfo,
        pos: Pos2,
        size: f32,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
        scale: f32,
        snarl: &Snarl<T>,
    ) -> Color32 {
        let _ = (pin, snarl);

        pin_info.draw(pos, size, snarl_style, style, painter, scale)
    }

    /// Draws background of the snarl view.
    ///
    /// By default it draws the background pattern using [`BackgroundPattern::draw`].
    ///
    /// If you want to draw the background yourself, you can override this method.
    fn draw_background(
        &mut self,
        background: Option<&BackgroundPattern>,
        viewport: &Viewport,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
        snarl: &Snarl<T>,
    ) {
        let _ = snarl;

        if let Some(background) = background {
            background.draw(viewport, snarl_style, style, painter)
        }
    }
}
