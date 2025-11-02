# screen_overlay

A Rust crate that allows drawing click-through overlays over full screen applications:

On Windows:
- Text.
- Bitmap images, support transparancy (tested with a png).
- Lines, circles, rectangles.

On X11:
- ~Text (no text wrapping).~
- It is now a full screen, click through window with a full opengl context. It doesn't support threading and will draw a single triangle. The `7f74c01a35d30529f37684402d5122eccd274c08` commit is the last one where text worked.

Mostly intended as a library to allow me to display text as a countdown from my behaviour tree library [betula](https://github.com/iwanders/betula/).

Another use case is adding a crosshair to games that don't have one, the `crosshair` example does this:
![crosshair](./examples/crosshair_image.png)

## License
License is `MIT OR Apache-2.0`.
