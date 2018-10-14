use glium::*;
use glium::index::*;
use lyon::*;
use lyon::tessellation::geometry_builder::*;
use lyon::math::*;
use lyon::lyon_tessellation::*;
use lyon::lyon_tessellation::basic_shapes::*;
use self::tessellation::{FillVertex, StrokeVertex};

pub const VERT: &str = include_str!("shaders/lines.vert");
pub const FRAG: &str = include_str!("shaders/lines.frag");

#[derive(Copy, Clone, Debug)]
struct Vertex2d {
    position: [f32; 2],
    color: [f32; 3]
}

implement_vertex!(Vertex2d, position, color);

struct Constructor {
    color: [f32; 3]
}

impl Constructor {
    fn new(color: [f32; 3]) -> Self {
        Self {
            color
        }
    }
}

impl VertexConstructor<FillVertex, Vertex2d> for Constructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex2d {
        Vertex2d {
            position: vertex.position.to_array(),
            color: self.color
        }
    }
}
impl VertexConstructor<StrokeVertex, Vertex2d> for Constructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex2d {
        Vertex2d {
            position: vertex.position.to_array(),
            color: self.color
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
            window_dimensions: [dimensions.width as f32, dimensions.height as f32]
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
            &mut BuffersBuilder::new(&mut self.vertex_buffers, Constructor::new([1.0; 3]))
        );
    }

    pub fn render_circle(&mut self, x: f32, y: f32, radius: f32, color: [f32; 3]) {        
        stroke_circle(
            point(x, y),
            radius,
            &self.stroke_options,
            &mut BuffersBuilder::new(&mut self.vertex_buffers, Constructor::new(color))
        );
    }
}

