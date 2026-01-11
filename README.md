# screen_overlay

This crate provides helpers and tooling to create transparent click-through overlays using [egui](https://github.com/emilk/egui/).

Originally (the `legacy` feature described below) this used raw X11 and Direct2D calls to draw text on the screen, but it's now
switched to just provide tooling around the `egui` ecosystem.

It is mostly intended as a library to allow me to display text as a countdown from my behaviour tree library [betula](https://github.com/iwanders/betula/).

It can also be used to draw a [full_screen_image](./examples/full_screen_image.rs) over the desktop, which can be useful to block out parts
during a screen capture.

Another use case is adding a crosshair to games that don't have one, the `crosshair` example does this:
![crosshair](./examples/crosshair_image.png).

It provides a thread safe `OverlayHandle` which can be shared between threads.
When an element is added to the overlay a handle is returned, if the handle goes out of scope the element is removed from the overlay.

## Legacy feature

The legacy implementation, hidden behind the `legacy` feature does the following:

A Rust crate that allows drawing click-through overlays over full screen applications:

On Windows:
- Text.
- Bitmap images, support transparancy (tested with a png).
- Lines, circles, rectangles.

On X11:
- ~Text (no text wrapping).~
- ~It is now a full screen, click through window with a full opengl context. It doesn't support threading and will draw a single triangle. The `7f74c01a35d30529f37684402d5122eccd274c08` commit is the last one where text worked.~
- Connected [three_d](https://github.com/asny/three-d) to the gl context with a transparant framebuffer, drawing lines works, but the transparent texture still doesn't.


## License
License is `MIT OR Apache-2.0`.
