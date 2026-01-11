use screen_overlay::{Drawable, Overlay, OverlayConfig, OverlayHandle, PositionedElements};

use clap::Parser;
use std::borrow::Cow;

/// A program capable of drawing a fullscreen image as an overlay.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg()]
    image_file: std::path::PathBuf,

    /// Width of the overlay.
    #[arg(short, long, default_value_t = 1920.0)]
    width: f32,

    /// Height of the overlay.
    #[arg(short, long, default_value_t = 1080.0)]
    height: f32,

    /// x position of the overlay.
    #[arg(short, long, default_value_t = 0.0)]
    x: f32,
    /// y position of the overlay.
    #[arg(short, long, default_value_t = 0.0)]
    y: f32,

    /// Use debug fill for the overlay.
    #[arg(short, long, default_value_t = false)]
    debug_fill: bool,
}
pub fn main() -> std::result::Result<(), eframe::Error> {
    let args = Args::parse();
    let config = OverlayConfig::new()
        .with_size([args.width, args.height])
        .with_position([args.x, args.y])
        .with_central_panel_fill(if args.debug_fill {
            screen_overlay::DEBUG_COLOR
        } else {
            screen_overlay::egui::Color32::TRANSPARENT
        });
    let overlay = Overlay::new(config);
    let overlay = OverlayHandle::new(overlay);

    let image_data = std::fs::read(&args.image_file).expect(&format!(
        "Failed to read image file at {}",
        args.image_file.display()
    ));
    let image_data = egui::load::Bytes::Shared(image_data.into());
    let image_path = args
        .image_file
        .file_name()
        .expect("no file_name part in provided path, is it a file?");

    let image_path = image_path.to_string_lossy().to_string();

    let image_source = egui::ImageSource::Bytes {
        uri: Cow::Owned(image_path),
        bytes: image_data.clone(),
    };

    let _image_on_screen = overlay.add_drawable(Drawable::CentralElement(
        PositionedElements::new()
            .fixed_pos(egui::pos2(0.0, 0.0))
            // .debug_color() // toggle this to see that this doesn't cover the entire area.
            .add_closure(move |ui| {
                ui.add(
                    egui::Image::new(image_source.clone()).fit_to_original_size(1.0), // prevents scaling to fit the size.
                );
            }),
    ));

    eframe::run_native(
        "Image Overlay",
        overlay.native_options(),
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(overlay))
        }),
    )
}
