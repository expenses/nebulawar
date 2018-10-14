use glium::*;
use glium::index::*;
use lyon::*;
use lyon::tessellation::geometry_builder::*;
use lyon::math::*;
use lyon::lyon_tessellation::*;
use lyon::lyon_tessellation::basic_shapes::*;
use self::tessellation::{FillVertex, StrokeVertex};

use super::*;

pub const VERT: &str = include_str!("shaders/lines.vert");
pub const FRAG: &str = include_str!("shaders/lines.frag");

#[derive(Copy, Clone, Debug)]
struct Vertex2d {
    position: [f32; 2],
    color: [f32; 4],
    uv: [f32; 2]
}

impl Vertex2d {
    fn new_image(position: [f32; 2], uv: [f32; 2]) -> Self {
        Self {
            position, uv,
            color: [0.0; 4]
        }
    }
}

implement_vertex!(Vertex2d, position, color, uv);

struct Constructor {
    color: [f32; 4]
}

impl Constructor {
    fn new(color: [f32; 4]) -> Self {
        Self {
            color
        }
    }
}

impl VertexConstructor<FillVertex, Vertex2d> for Constructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex2d {
        Vertex2d {
            position: vertex.position.to_array(),
            color: self.color,
            uv: [0.0; 2]
        }
    }
}
impl VertexConstructor<StrokeVertex, Vertex2d> for Constructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex2d {
        Vertex2d {
            position: vertex.position.to_array(),
            color: self.color,
            uv: [0.0; 2]
        }
    }
}

pub struct LineRenderer {
    program: Program,
    stroke_options: StrokeOptions,
    vertex_buffers: VertexBuffers<Vertex2d, u16>
}

impl LineRenderer {
    pub fn new(display: &Display) -> Self {
        Self {
            program: Program::from_source(
                display,
                VERT, FRAG,
                None
            ).unwrap(),
            stroke_options: StrokeOptions::tolerance(1.0).with_line_width(1.0),            
            vertex_buffers: VertexBuffers::new()
        }
    }

    pub fn flush(&mut self, target: &mut Frame, display: &Display) {
        let dimensions = display.gl_window().get_inner_size().unwrap();

        let uniforms = uniform!{
            window_dimensions: [dimensions.width as f32, dimensions.height as f32],
            draw_image: false
        };

        let draw_params = DrawParameters {
            blend: Blend::alpha_blending(),
            .. Default::default()
        };

        let vertices = VertexBuffer::new(display, &self.vertex_buffers.vertices).unwrap();
        let indices = IndexBuffer::new(display, PrimitiveType::TrianglesList, &self.vertex_buffers.indices).unwrap();

        target.draw(&vertices, &indices, &self.program, &uniforms, &draw_params).unwrap();

        self.vertex_buffers.vertices.clear();
        self.vertex_buffers.indices.clear();
    }

    pub fn render_rect(&mut self, top_left: (f32, f32), bottom_right: (f32, f32)) {
        let (left, top) = top_left;
        let (right, bottom) = bottom_right;

        stroke_quad(
            point(left, top),
            point(right, top),
            point(right, bottom),
            point(left, bottom),
            &self.stroke_options,
            &mut BuffersBuilder::new(&mut self.vertex_buffers, Constructor::new([1.0; 4]))
        );
    }

    pub fn render_circle(&mut self, x: f32, y: f32, radius: f32, color: [f32; 4]) {        
        stroke_circle(
            point(x, y),
            radius,
            &self.stroke_options,
            &mut BuffersBuilder::new(&mut self.vertex_buffers, Constructor::new(color))
        );
    }

    pub fn render_image(&self, image: Image, x: f32, y: f32, width: f32, height: f32, target: &mut Frame, display: &Display, resources: &Resources) {
        let dimensions = display.gl_window().get_inner_size().unwrap();

        let uniforms = uniform!{
            window_dimensions: [dimensions.width as f32, dimensions.height as f32],
            image: Sampler::new(&resources.images[image as usize])
                .minify_filter(MinifySamplerFilter::Nearest)
                .magnify_filter(MagnifySamplerFilter::Nearest),
            draw_image: true
        };

        let draw_params = DrawParameters {
            blend: Blend::alpha_blending(),
            .. Default::default()
        };
        
        let vertices = [
            Vertex2d::new_image([x - width / 2.0, y - height / 2.0], [0.0, 1.0]),
            Vertex2d::new_image([x + width / 2.0, y - height / 2.0], [1.0, 1.0]),
            Vertex2d::new_image([x - width / 2.0, y + height / 2.0], [0.0, 0.0]),
            Vertex2d::new_image([x + width / 2.0, y + height / 2.0], [1.0, 0.0])
        ];

        let vertices = VertexBuffer::new(display, &vertices).unwrap();
        let indices = NoIndices(PrimitiveType::TriangleStrip);

        target.draw(&vertices, &indices, &self.program, &uniforms, &draw_params).unwrap();
    }
}

