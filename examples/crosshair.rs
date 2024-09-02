
use screen_overlay::{
    Color, DrawGeometry, GeometryElement, Point, Rect, Stroke, TextAlignment, TextProperties, LineStyle, CapStyle, LineJoin,DashStyle, CircleDirection, OverlayConfig, Overlay, Error
};

pub fn main() -> std::result::Result<(), Error> {
    screen_overlay::setup()?;
    let window = Overlay::new_with_config(&OverlayConfig{
        task_bar: true,
        on_top: true,
        name: "Crosshair".to_owned(),
        ..Default::default()
    })?;

    let pixel_offset = 0.5;
    let crosshair_pos = Point::new(1920.0 / 2.0 + pixel_offset, 1080.0 / 2.0 + pixel_offset);

    let size = 10.0;
    let top = crosshair_pos + Point::new(0.0, -size);
    let below = crosshair_pos + Point::new(0.0, size);

    let left = crosshair_pos + Point::new(-size, 0.0);
    let right = crosshair_pos + Point::new(size, 0.0);


    let geometry = DrawGeometry::new().line_segment(&top, &below).line_segment(&left, &right);
    println!("geometry: {geometry:?}");
    let color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 128,
    };
    let stroke = Stroke { color, width: 5.0 };


    let _ = window.draw_geometry(&geometry, &stroke, &Default::default())?;

    Ok(screen_overlay::block_and_loop()?)
}
