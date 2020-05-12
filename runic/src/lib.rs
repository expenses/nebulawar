// Disable the clippy lint about having too many function arguments
#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

extern crate rusttype;
extern crate glium;

mod tests;
mod cache;

pub use cache::GlyphCache;
use cache::*;

use rusttype::*;
use rusttype::gpu_cache::*;
use glium::texture::*;
use glium::uniforms::*;
use glium::backend::*;
use glium::*;

use std::borrow::Cow;
use std::fmt;

/// The default vertex shader.
pub const VERT: &str = include_str!("shaders/shader.vert");
/// The default fragmentation shader.
pub const FRAG: &str = include_str!("shaders/shader.frag");

/// Get the default program for rendering glyphs, which just renders them with a solid colour.
pub fn default_program<F: Facade>(display: &F) -> Result<Program, ProgramCreationError> {
    Program::from_source(display, VERT, FRAG, None)
}

fn screen_pos_to_opengl_pos(position: [f32; 2], screen_width: f32, screen_height: f32) -> [f32; 2] {
    [
        (position[0] / screen_width - 0.5) * 2.0,
        (1.0 - position[1] / screen_height - 0.5) * 2.0
    ]
} 

fn screen_rect_to_opengl_rect(rect: rusttype::Rect<i32>, screen_width: f32, screen_height: f32, origin: [f32; 2]) -> rusttype::Rect<f32> {
    // add the origin to the rectangle
    
    let min = [
        rect.min.x as f32 + origin[0],
        rect.min.y as f32 + origin[1] 
    ];

    let max = [
        rect.max.x as f32 + origin[0],
        rect.max.y as f32 + origin[1] 
    ];

    let min = screen_pos_to_opengl_pos(min, screen_width, screen_height);
    let max = screen_pos_to_opengl_pos(max, screen_width, screen_height);

    rusttype::Rect {
        min: point(min[0], min[1]),
        max: point(max[0], max[1])
    }
}

/// A vertex for rendering.
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub in_pos: [f32; 2],
    pub in_uv: [f32; 2],
}

implement_vertex!(Vertex, in_pos, in_uv);

/// Get the logical height of a piece of rendered text.
///
/// This is actually just the scaled 'ascent', or highest point of any character, of the font,
/// not the height of just the characters in the string.
pub fn rendered_height(font: &Font, scale: f32) -> f32 {
    font.v_metrics(Scale::uniform(scale)).ascent
}

/// Get the logical width of a piece of rendered text.
pub fn rendered_width<D: Display>(text: &str, scale: f32, font: &Font, pixelated: bool, display: &D) -> f32 {
    let dpi = display.dpi_factor();

    // Find the rightmost bounding box edge
    layout_glyphs(text, scale, dpi, font, pixelated)
        .filter_map(|glyph| glyph.pixel_bounding_box())
        .fold(0.0_f32, |width, bbox| width.max(bbox.max.x as f32))
}

/// A font with a contained glyph cache for ease of use.
pub struct CachedFont<'a> {
    font: Font<'a>,
    cache: GlyphCache
}

impl<'a> CachedFont<'a> {
    /// Setup the struct and create the glyph cache.
    pub fn new<D: Display>(font: Font<'a>, display: &D) -> Result<Self, Error> {
        Ok(Self {
            font,
            cache: GlyphCache::new(display)?
        })
    }

    /// Read a font from bytes and create the cache.
    pub fn from_bytes<D: Display>(bytes: &'a [u8], display: &D) -> Result<Self, Error> {
        Self::new(
            Font::from_bytes(bytes).map_err(Error::Font)?,
            display
        )
    }

    /// Get the contained font.
    pub fn inner(&self) -> &Font<'a> {
        &self.font
    }

    /// Get the contained cache
    pub fn cache(&self) -> &GlyphCache {
        &self.cache
    }

    /// Render the font onto a target via shaders.
    ///
    /// See [`GlyphCache::get_vertices`] for infomation on the arguments.
    ///
    /// [`GlyphCache::get_vertices`]: struct.GlyphCache.html#method.get_vertices
    pub fn render<S: Surface, D: Display>(&mut self, text: &str, origin: [f32; 2], scale: f32, colour: [f32; 4], target: &mut S, display: &D, program: &Program) -> Result<(), Error> {
        let vertices: Vec<_> = self.cache.get_vertices(text, origin, scale, &self.font, 0, false, display)?.collect();
        self.cache.render_vertices(&vertices, colour, target, display, program, false)
    }

    pub fn render_pixelated<S: Surface, D: Display>(&mut self, text: &str, origin: [f32; 2], font_size: f32, scale: f32, colour: [f32; 4], target: &mut S, display: &D, program: &Program) -> Result<(), Error> {
        let vertices: Vec<_> = self.cache.get_pixelated_vertices(text, origin, font_size, scale, &self.font, 0, display)?.collect();
        self.cache.render_vertices(&vertices, colour, target, display, program, true)
    }

    pub fn get_vertices<'b, D: 'b + Display>(&'b mut self, text: &'b str, origin: [f32; 2], scale: f32, pixelated: bool, display: &D) -> Result<impl Iterator<Item=Vertex> + 'b, Error> {
        self.cache.get_vertices(text, origin, scale, &self.font, 0, pixelated, display)
    }

    pub fn rendered_width<D: Display>(&self, text: &str, scale: f32, pixelated: bool, display: &D) -> f32 {
        rendered_width(text, scale, &self.font, pixelated, display)
    }

    pub fn rendered_height(&self, scale: f32) -> f32 {
        rendered_height(&self.font, scale)
    }

    pub fn render_vertices<S: Surface, D: Display>(&self, vertices: &[Vertex], colour: [f32; 4], target: &mut S, display: &D, program: &Program, pixelated: bool) -> Result<(), Error> {
        self.cache.render_vertices(vertices, colour, target, display, program, pixelated)
    }

    pub fn get_pixelated_vertices<'b, D: 'b + Display>(&'b mut self, text: &'b str, origin: [f32; 2], font_size: f32, scale: f32, display: &D) -> Result<impl Iterator<Item=Vertex> + 'b, Error> {
        self.cache.get_pixelated_vertices(text, origin, font_size, scale, &self.font, 0, display)
    }
}

#[derive(Debug)]
/// All the errors that can occur.
pub enum Error {
    CacheWrite(CacheWriteErr),
    BufferCreation(vertex::BufferCreationError),
    Draw(DrawError),
    Font(rusttype::Error),
    TextureCreation(TextureCreationError)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::CacheWrite(error) => error.fmt(f),
            Error::BufferCreation(error) => error.fmt(f),
            Error::Draw(error) => error.fmt(f),
            Error::Font(error) => error.fmt(f),
            Error::TextureCreation(error) => error.fmt(f)
        }
    }
}

impl std::error::Error for Error {}

pub trait Display: Facade + Sized {
    fn dpi_factor(&self) -> f32;
    fn framebuffer_dimensions(&self) -> (u32, u32);
}

impl Display for glium::Display {
    fn dpi_factor(&self) -> f32 {
        self.gl_window().window().scale_factor() as f32
    }

    fn framebuffer_dimensions(&self) -> (u32, u32) {
        self.get_framebuffer_dimensions()
    }
}

impl Display for glium::backend::glutin::headless::Headless {
    fn dpi_factor(&self) -> f32 {
        1.5
    }

    fn framebuffer_dimensions(&self) -> (u32, u32) {
        self.get_framebuffer_dimensions()
    }
}
