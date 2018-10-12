use glium::*;
use glium::index::*;
use arrayvec::*;

#[derive(Copy, Clone, Debug)]
struct Vert {
    pos: [f32; 2],
}

use lyon::*;
use lyon::tessellation::geometry_builder::*;
use lyon::math::*;
use lyon::lyon_tessellation::*;
use lyon::lyon_tessellation::basic_shapes::*;

struct VertConstructor;

impl VertexConstructor<tessellation::FillVertex, Vert> for VertConstructor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> Vert {
        Vert { pos: vertex.position.to_array(), }
    }
}
impl VertexConstructor<tessellation::StrokeVertex, Vert> for VertConstructor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> Vert {
        Vert { pos: vertex.position.to_array(), }
    }
}

implement_vertex!(Vert, pos);

pub struct LineRenderer {
    program: Program,
    stroke_options: StrokeOptions,
    vertex_buffers: VertexBuffers<Vert, u16>
}

impl LineRenderer {
    pub fn new(display: &Display) -> Self {
        Self {
            program: Program::from_source(
                display,
                include_str!("shaders/lines.vert"),
                include_str!("shaders/lines.frag"),
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

    pub fn render_line(&mut self, start: (f32, f32), end: (f32, f32)) {
        let (start_x, start_y) = start;
        let (end_x, end_y) = end;
        
        stroke_polyline(
            ArrayVec::from([point(start_x, start_y), point(end_x, end_y)]).into_iter(),
            false,
            &self.stroke_options,
            &mut BuffersBuilder::new(&mut self.vertex_buffers, VertConstructor)
        );
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
            &mut BuffersBuilder::new(&mut self.vertex_buffers, VertConstructor)
        );
    }

    pub fn render_circle(&mut self, x: f32, y: f32, radius: f32) {        
        stroke_circle(
            point(x, y),
            radius,
            &self.stroke_options,
            &mut BuffersBuilder::new(&mut self.vertex_buffers, VertConstructor)
        );
    }
}