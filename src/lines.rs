use line_drawing::*;
use arrayvec::*;

use glium::*;
use glium::index::*;

#[derive(Copy, Clone, Debug)]
struct Vert {
    pos: [f32; 2],
}

implement_vertex!(Vert, pos);

pub struct LineRenderer {
    program: Program
}

impl LineRenderer {
    pub fn new(display: &Display) -> Self {
        Self {
            program: Program::from_source(
                display,
                include_str!("lines.vert"),
                include_str!("lines.frag"),
                None
            ).unwrap()
        }
    }

    fn render_iter<I: Iterator<Item=(i32, i32)>>(&self, iter: I, target: &mut Frame, display: &Display) {
        let vertices: Vec<Vert> = iter
            .flat_map(|(x, y)| {
                let tl = Vert {
                    pos: [x as f32, y as f32],
                };

                let tr = Vert {
                    pos: [x as f32 + 1.0, y as f32],
                };

                let bl = Vert {
                    pos: [x as f32, y as f32 + 1.0],
                };

                let br = Vert {
                    pos: [x as f32 + 1.0, y as f32 + 1.0],
                };

                ArrayVec::from([tl, tr, bl, tr, bl, br])
            })
            .collect();

        let buffer = VertexBuffer::new(display, &vertices).unwrap();

        let dimensions = display.gl_window().get_inner_size().unwrap();

        let uniforms = uniform!{
            window_dimensions: [dimensions.width as f32, dimensions.height as f32]
        };

        let draw_params = DrawParameters {
            blend: Blend::alpha_blending(),
            .. Default::default()
        };

        target.draw(&buffer, index::NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &draw_params).unwrap();
    }

    pub fn render_line(&self, start: (i32, i32), end: (i32, i32), target: &mut Frame, display: &Display) {
        self.render_iter(Bresenham::new(start, end), target, display);
    }

    pub fn render_circle(&self, center: (i32, i32), radius: i32, target: &mut Frame, display: &Display) {
        self.render_iter(BresenhamCircle::new(center.0, center.1, radius), target, display);
    }
}