use screen_overlay::{
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

pub fn main() -> std::result::Result<(), Error> {
    screen_overlay::setup()?;
    let window = Overlay::new_with_config(&OverlayConfig {
        name: "Awesome Overlay".to_owned(),
        ..Default::default()
    })?;

    let twindow = window.clone();
    let _msg_loop_thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let color = Color {
            r: 255,
            g: 0,
            b: 255,
            a: 128,
        };
        let alpha = 0.5;
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

        let image = twindow
            .load_texture(std::path::PathBuf::from("image.jpg"))
            .expect("failed to load image");
        let _t = twindow
            .draw_texture(
                &Point::new(1000.0, 500.0),
                &image,
                &Rect::from(0.0, 0.0).sized(200.0, 200.0),
                &Color::TRANSPARENT,
                alpha,
            )
            .expect("texture draw failed");

        for i in 0..50000 {
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

            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

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

        println!("blocking now");
        std::thread::sleep(std::time::Duration::from_millis(1000000));
    });
    Ok(screen_overlay::block_and_loop()?)

    // Ok(())
}
