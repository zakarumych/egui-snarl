# egui-snarl

Crate for creating node-graph UIs.
This is in early development. Many more features are planned.

# Why "snarl"?

Because that's how any complex graph UI looks like.

# Features

- Typed data-only nodes.
  `Snarl` is parametrized by the type of the data nodes hold.
  In typical scenario node type would be an enum to hold different kinds of the nodes.

- Viewer trait to define behavior and add extra data to UI routine.
  `SnarlViewer` trait is parametrized by the type node type.
  It decides node's title UI, how many pins node has and fills pin's UI content.
  Demo example showcase how pin can have drag integer value, text input, button or image,
  there's no limitations since each pin's content is whatever viewer puts in provided `egui::Ui`.

- Serialization.
  `Snarl` structure avoids storing anything but the graph with placed nodes and wires between them.
  This makes it suitable for serialization and deserialization.
  It supports `serde` so pick your own format.

# Example

`demo` example shows some of the features of the crate.

[![demo](./demo.png)](./demo.png)
