mod lines;
mod resources;

use self::resources::*;
use self::lines::*;

pub use self::resources::Model;

use cgmath::*;

use {
    glium,
    glium::{
        *,
       uniforms::*,
       index::*,
       texture::*
    }
};

use camera::*;
use *;

use runic;

pub enum Mode {
    Normal = 1,
    Shadeless = 2,
    Stars = 3,
    Wireframe = 4
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture: [f32; 2],
}

implement_vertex!(Vertex, position, normal, texture);

pub struct Context {
    display: Display,
    program: Program,
    target: Frame,
    resources: Resources,
    lines: LineRenderer,
    text_program: Program
}

impl Context {
    pub fn new(events_loop: &EventsLoop) -> Self {
        let window = glutin::WindowBuilder::new();
        let context = glutin::ContextBuilder::new()
            .with_multisampling(16)
            .with_depth_buffer(24)
            .with_vsync(true);
        
        let display = glium::Display::new(window, context, &events_loop).unwrap();

        let program = glium::Program::from_source(
                &display,
                include_str!("shaders/shader.vert"),
                include_str!("shaders/shader.frag"),
                None
        ).unwrap();

        Self {
            resources: Resources::new(&display),
            target: display.draw(),
            program,
            lines: LineRenderer::new(&display),
            text_program: runic::pixelated_program(&display).unwrap(),
            display
        }
    }

    pub fn clear(&mut self, system: &System) {
        let (r, g, b) = system.background_color;
        self.target.clear_color_srgb_and_depth((r, g, b, 1.0), 1.0);
    }

    pub fn flush_ui(&mut self) {
        self.lines.flush(&mut self.target, &self.display);
    }

    pub fn finish(&mut self) {
        self.target.set_finish().unwrap();
        self.target = self.display.draw();
    }

    fn render_billboard(&mut self, matrix: Matrix4<f32>, image: Image, camera: &Camera, system: &System) {
        let uniforms = self.uniforms(matrix, camera, system, &self.resources.images[image as usize], Mode::Shadeless);

        self.target.draw(
            &billboard(&self.display),
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &Self::draw_params()
        ).unwrap();
    }

    pub fn render_system(&mut self, system: &System, camera: &Camera) {
        let uniforms = uniform!{
            model: matrix_to_array(Matrix4::identity()),
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            mode: Mode::Stars as i32
        };

        let vertices: Vec<Vertex> = system.stars.iter()
            .map(|(vector, brightness)| {
                context::Vertex {
                    position: (camera.position() + vector * 1010.0).into(),
                    normal: [0.0; 3],
                    texture: [*brightness; 2]
                }
            })
            .collect();

        let buffer = VertexBuffer::new(&self.display, &vertices).unwrap();
        let indices = NoIndices(PrimitiveType::Points);

        let params = DrawParameters {
            polygon_mode: PolygonMode::Point,
            point_size: Some(2.0 * self.dpi()),
            .. Self::draw_params()
        };

        self.target.draw(&buffer, &indices, &self.program, &uniforms, &params).unwrap();

        let offset = system.light * 1000.0;

        let rotation: Matrix4<f32> = Quaternion::look_at(offset, Vector3::new(0.0, 1.0, 0.0)).invert().into();
        let matrix = Matrix4::from_translation(camera.position() + offset) * rotation * Matrix4::from_scale(100.0);

        self.render_billboard(matrix, Image::Star, camera, system);
    }

    pub fn uniforms<'a>(&self, position: Matrix4<f32>, camera: &Camera, system: &System, texture: &'a SrgbTexture2d, mode: Mode) -> impl Uniforms + 'a {
        uniform!{
            model: matrix_to_array(position),
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            tex: Sampler::new(texture).minify_filter(MinifySamplerFilter::Nearest).magnify_filter(MagnifySamplerFilter::Nearest),
            mode: mode as i32
        }
    }

    pub fn render(&mut self, model: Model, position: Matrix4<f32>, camera: &Camera, system: &System, mode: Mode) {
        let model = &self.resources.models[model as usize];

        let uniforms = self.uniforms(position, camera, system, &model.texture, mode);

        let params = Self::draw_params();

        self.target.draw(&model.vertices, &NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();
    }

    pub fn render_text(&mut self, text: &str, x: f32, y: f32) {
        self.resources.font.render_pixelated(text, [x, y], 16.0, 1.0, [1.0; 4], &mut self.target, &self.display, &self.text_program).unwrap();
    }

    pub fn render_rect(&mut self, top_left: (f32, f32), bottom_right: (f32, f32)) {
        self.lines.render_rect(top_left, bottom_right);
    }

    pub fn render_line(&mut self, start: (f32, f32), end: (f32, f32)) {
        self.lines.render_line(start, end);
    }

    pub fn render_circle(&mut self, x: f32, y: f32, radius: f32, color: [f32; 3]) {
        self.lines.render_circle(x, y, radius, color);
    }

    fn screen_dimensions(&self) -> (f32, f32) {
        let (width, height) = self.target.get_dimensions();
        (width as f32, height as f32)
    }

    fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.screen_dimensions();
        height / width
    }

    fn perspective_matrix(&self) -> Matrix4<f32> {
        perspective_matrix(self.aspect_ratio())
    }

    fn dpi(&self) -> f32 {
        self.display.gl_window().get_hidpi_factor() as f32
    }

    // Get the screen position of a point if it is in front of the camera
    pub fn screen_position(&self, point: Vector3<f32>, camera: &Camera) -> Option<(f32, f32)> {
        let modelview = camera.view_matrix() * Matrix4::from_translation(point);

        let gl_position = self.perspective_matrix() * modelview * Vector4::new(0.0, 0.0, 0.0, 1.0);

        let x = gl_position[0] / gl_position[3];
        let y = gl_position[1] / gl_position[3];
        let z = gl_position[2] / gl_position[3];

        let (width, height) = self.screen_dimensions();
        let (x, y) = opengl_pos_to_screen_pos(x, y, width, height);
        let (x, y) = (x * 2.0 / self.dpi(), y * 2.0 / self.dpi());

        if z < 1.0 {
            Some((x, y))
        } else {
            None
        }
    }

    fn draw_params() -> DrawParameters<'static> {
        DrawParameters {
            depth: Depth {
                test: DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: BackfaceCullingMode::CullCounterClockwise,
            blend: Blend::alpha_blending(),
            .. Default::default()
        }
    }

    pub fn render_3d_line(&mut self, start: Vector3<f32>, end: Vector3<f32>, camera: &Camera, system: &System) {
        let uniforms = uniform!{
            model: matrix_to_array(Matrix4::identity()),
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            mode: Mode::Stars as i32
        };

        let vertices = [
            Vertex {
                position: start.into(),
                normal: [1.0; 3],
                texture: [1.0; 2]
            },
            Vertex {
                position: end.into(),
                normal: [0.0; 3],
                texture: [1.0; 2]
            }
        ];

        let buffer = VertexBuffer::new(&self.display, &vertices).unwrap();
        let indices = NoIndices(PrimitiveType::LinesList);

        let params = DrawParameters {
            polygon_mode: PolygonMode::Line,
            .. Self::draw_params()
        };

        self.target.draw(&buffer, &indices, &self.program, &uniforms, &params).unwrap();
    }

    // http://webglfactory.blogspot.com/2011/05/how-to-convert-world-to-screen.html
    pub fn ray(&self, camera: &Camera, mouse: (f32, f32)) -> collision::Ray<f32, Point3<f32>, Vector3<f32>> {
        // Get mouse position
        let (x, y) = mouse;
        // Not sure why we have to do this
        let (x, y) = (x * self.dpi(), y * self.dpi());

        let (width, height) = self.screen_dimensions();
        let (x, y) = screen_pos_to_opengl_pos(x, y, width, height);
        let point = Vector4::new(x, y, 1.0, 1.0);

        // Invert the perspective/view matrix
        let perspective_view_inverse = (self.perspective_matrix() * camera.view_matrix()).invert().unwrap();
        // Multiply the point by it and truncate
        let point = (perspective_view_inverse * point).truncate();

        // Create a ray from the camera position to that point
        collision::Ray::new(vector_to_point(camera.position()), point)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        self.target.set_finish().unwrap();
    }
}