#![allow(unused_variables, unused_imports, unreachable_code)]
use crate::{
    CapStyle, CircleDirection, Color, DashStyle, DrawGeometry, Error, GeometryElement, LineJoin,
    LineStyle, OverlayConfig, Point, Rect, Stroke, TextAlignment, TextProperties,
};

/*
    We can probably draw on https://github.com/ftorkler/x11-overlay for a lot of the logic.
*/

use std::sync::Arc;

#[derive(Clone)]
pub struct ImageTexture {}
impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ImageTexture {:?}", &self)
    }
}

#[derive(Clone)]
pub struct PreparedFont {}
impl std::fmt::Debug for PreparedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "PreparedFont {:?}", &self)
    }
}

pub type IDVisual = usize;

pub struct OverlayImpl {}

impl OverlayImpl {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {})
    }

    pub fn create_window(&mut self, config: &OverlayConfig) -> Result<(), Error> {
        Ok(())
    }

    pub fn create_device_resources(&mut self) -> Result<(), Error> {
        Ok(())
    }

    pub fn draw_geometry(
        &mut self,
        geometry: &DrawGeometry,
        stroke: &Stroke,
        line_style: &LineStyle,
    ) -> Result<IDVisual, Error> {
        Ok(3)
    }

    pub fn prepare_font(&mut self, properties: &TextProperties) -> Result<PreparedFont, Error> {
        Ok(PreparedFont {})
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        layout: &Rect,
        color: &Color,
        font: &PreparedFont,
    ) -> Result<IDVisual, Error> {
        println!("would print {text}");
        Ok(3)
    }

    pub fn load_texture<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<ImageTexture, Error> {
        Ok(ImageTexture {})
    }

    pub fn draw_texture(
        &mut self,
        position: &Point,
        texture: &ImageTexture,
        texture_region: &Rect,
        color: &Color,
        alpha: f32,
    ) -> Result<IDVisual, Error> {
        Ok(3)
    }

    pub fn remove_visual(&mut self, visual: &IDVisual) -> Result<(), Error> {
        Ok(())
    }
}
// Is this legal?
// unsafe impl Send for OverlayImpl {}
pub fn run_msg_loop() -> Result<(), Error> {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}

pub fn setup() -> Result<(), Error> {
    Ok(())
}
