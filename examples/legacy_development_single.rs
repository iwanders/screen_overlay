#[cfg(not(feature = "legacy"))]
pub fn main() {}

#[cfg(feature = "legacy")]
use screen_overlay::legacy::{
    // CapStyle, CircleDirection, GeometryElement, LineJoin,
    Color,
    DashStyle,
    DrawGeometry,
    Error,
    LineStyle,
    Overlay,
    OverlayConfig,
    Point,
    Rect,
    Stroke,
    TextAlignment,
    TextProperties,
};

#[cfg(feature = "legacy")]
pub fn main() -> std::result::Result<(), Error> {
    let v = screen_overlay::legacy::setup()?;
    let window = Overlay::new_with_config(&OverlayConfig {
        name: "Awesome Overlay".to_owned(),
        ..Default::default()
    })?;

    let twindow = window.clone();

    let color = Color {
        r: 255,
        g: 0,
        b: 255,
        a: 128,
    };
    let alpha = 0.5;
    println!("first image alloc");
    let _image1 = twindow
        .load_texture(std::path::PathBuf::from(
            "PNG_transparency_demonstration_1.png",
        ))
        .expect("failed to load image");

    println!("second image alloc");
    let image2 = twindow
        .load_texture(std::path::PathBuf::from(
            "PNG_transparency_demonstration_1.png",
        ))
        .expect("failed to load image");
    let _t2 = twindow
        .draw_texture(
            &Point::new(500.0, 500.0),
            &image2,
            &Rect::from(0.0, 0.0).sized(200.0, 200.0),
            &color,
            alpha,
        )
        .expect("texture draw failed");

    #[cfg(target_os = "linux")]
    twindow.render();
    /*
        let i = 300;
        let pos = Rect::from(200.0 + 50.0 * (i % 5) as f32, 200.0).sized(200.0, 300.0);
        let geometry = DrawGeometry::new().rectangle(&pos);
        let color = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        let stroke = Stroke { color, width: 1.0 };
        let text_box_style = LineStyle {
            dash_style: DashStyle::Dash,
            // line_join: LineJoin::Round,
            ..Default::default()
        };

        let _v = twindow
            .draw_geometry(&geometry, &stroke, &text_box_style)
            .expect("create image failed");

        let font = twindow
            .prepare_font(&TextProperties {
                size: 32.0,
                horizontal_align: TextAlignment::Min,
                vertical_align: TextAlignment::Min,
                ..Default::default()
            })
            .expect("preparing the font failed");

        let color = Color {
            r: 255,
            g: 0,
            b: 255,
            a: 128,
        };
        let _v = twindow
            .draw_text("hello there we are rendering text", &pos, &color, &font)
            .expect("create image failed");

        let geometry = DrawGeometry::new().circle(&Point::new(510.0, 500.0), 10.0);
        let color = Color {
            r: 0,
            g: 255,
            b: 255,
            a: 255,
        };
        let stroke = Stroke { color, width: 1.0 };

        let _v = twindow
            .draw_geometry(&geometry, &stroke, &Default::default())
            .expect("create image failed");

        let _z = {
            let geometry = DrawGeometry::new()
                .hollow(200.0, 10.0)
                .line(100.0, 100.0)
                .closed();
            let color = Color {
                r: 255,
                g: 0,
                b: 255,
                a: 255,
            };
            let stroke = Stroke { color, width: 30.0 };

            let v = twindow
                .draw_geometry(&geometry, &stroke, &Default::default())
                .expect("create image failed");
            std::thread::sleep(std::time::Duration::from_millis(500));
            v
        };
    */
    Ok(screen_overlay::legacy::block_and_loop(v)?)

    // Ok(())
}
