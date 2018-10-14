mod lines;
mod resources;

use self::lines::*;

pub use self::resources::*;

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
use pedot::*;

// ** Line Rendering Woes **
// rendering in 2d: doesnt work with rest of scene, rendering lines that go behind the camera is hard
// gl_lines: has a max width of 1 on my laptop
// 2d lines in 3d: getting lines to join nicely is hard, too flat
// geometry shader: complicated
// assembling triangle/square line meshes by hand: complicated, but might be best shot

const VERT: &str = include_str!("shaders/shader.vert");
const FRAG: &str = include_str!("shaders/shader.frag");

const DEFAULT_WIDTH: f32 = 800.0;
const DEFAULT_HEIGHT: f32 = 600.0;

pub enum Mode {
    Normal = 1,
    Shadeless = 2,
    Stars = 3
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture: [f32; 2],
}

impl Vertex {
    fn new(position: Vector3<f32>) -> Self {
        Self {
            position: position.into(),
            normal: [0.0; 3],
            texture: [1.0; 2]
        }
    }
}

implement_vertex!(Vertex, position, normal, texture);

pub struct Context {
    display: Display,
    program: Program,
    target: Frame,
    pub resources: Resources,
    lines: LineRenderer,
    text_program: Program,
    lines_3d: Vec<Vertex>,
    debug: bool,
    pub gui: Gui
}

impl Context {
    pub fn new(events_loop: &EventsLoop) -> Self {
        let window = glutin::WindowBuilder::new()
            .with_dimensions(LogicalSize::new(DEFAULT_WIDTH as f64, DEFAULT_HEIGHT as f64))
            .with_title("Fleet Commander");
        let context = glutin::ContextBuilder::new()
            .with_multisampling(16)
            .with_depth_buffer(24)
            .with_vsync(true);
        
        let display = glium::Display::new(window, context, &events_loop).unwrap();

        let program = glium::Program::from_source(
                &display,
                VERT, FRAG,
                None
        ).unwrap();

        Self {
            resources: Resources::new(&display),
            target: display.draw(),
            lines: LineRenderer::new(&display),
            text_program: runic::pixelated_program(&display).unwrap(),
            display, program,
            lines_3d: Vec::new(),
            debug: false,
            gui: Gui::new(DEFAULT_WIDTH, DEFAULT_HEIGHT)
        }
    }

    pub fn copy_event(&mut self, event: &WindowEvent) {
        self.gui.update(event);
    }

    pub fn clear(&mut self, system: &System) {
        let (r, g, b) = system.background_color;
        self.target.clear_color_srgb_and_depth((r, g, b, 1.0), 1.0);
    }

    pub fn flush_ui(&mut self, camera: &Camera, system: &System) {
        self.lines.flush(&mut self.target, &self.display);

        let uniforms = uniform!{
            model: matrix_to_array(Matrix4::identity()),
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            // todo: this is kinda lazy, so maybe change the name of the mode
            mode: Mode::Stars as i32
        };

        let vertices = VertexBuffer::new(&self.display, &self.lines_3d).unwrap();
        let indices = NoIndices(PrimitiveType::LinesList);

        let params = self.draw_params();
        self.target.draw(&vertices, &indices, &self.program, &uniforms, &params).unwrap();

        self.lines_3d.clear();

        self.gui.clear();
    }

    pub fn finish(&mut self) {
        self.target.set_finish().unwrap();
        self.target = self.display.draw();
    }

    fn render_billboard(&mut self, matrix: Matrix4<f32>, image: Image, camera: &Camera, system: &System) {
        let uniforms = self.uniforms(matrix, camera, system, &self.resources.images[image as usize], Mode::Shadeless);
        let params = self.draw_params();

        self.target.draw(
            &billboard(&self.display),
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &params
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

        let vertices = VertexBuffer::new(&self.display, &vertices).unwrap();
        let indices = NoIndices(PrimitiveType::Points);

        let params = DrawParameters {
            polygon_mode: PolygonMode::Point,
            point_size: Some(2.0 * self.dpi()),
            .. Self::draw_params(self)
        };

        self.target.draw(&vertices, &indices, &self.program, &uniforms, &params).unwrap();

        let offset = system.light * 1000.0;

        let rotation: Matrix4<f32> = look_at(offset).into();
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
        let params = self.draw_params();

        self.target.draw(&model.vertices, &NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();
    }

    pub fn render_text(&mut self, text: &str, x: f32, y: f32) {
        self.resources.font.render_pixelated(text, [x, y], 16.0, 1.0, [1.0; 4], &mut self.target, &self.display, &self.text_program).unwrap();
    }

    pub fn render_rect(&mut self, top_left: (f32, f32), bottom_right: (f32, f32)) {
        self.lines.render_rect(top_left, bottom_right);
    }

    pub fn render_circle(&mut self, x: f32, y: f32, radius: f32, color: [f32; 4]) {
        self.lines.render_circle(x, y, radius, color);
    }

    pub fn screen_dimensions(&self) -> (f32, f32) {
        let dimensions = self.display.gl_window().get_inner_size().unwrap();
        (dimensions.width as f32, dimensions.height as f32)
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
    pub fn screen_position(&self, point: Vector3<f32>, camera: &Camera) -> Option<(f32, f32, f32)> {
        let modelview = camera.view_matrix() * Matrix4::from_translation(point);

        let gl_position = self.perspective_matrix() * modelview * Vector4::new(0.0, 0.0, 0.0, 1.0);

        let x = gl_position[0] / gl_position[3];
        let y = gl_position[1] / gl_position[3];
        let z = gl_position[2] / gl_position[3];

        let (width, height) = self.screen_dimensions();
        let (x, y) = opengl_pos_to_screen_pos(x, y, width, height);
        // todo: this may be dpi dependent
        let (x, y) = (x * 2.0, y * 2.0);

        if z < 1.0 {
            Some((x, y, z))
        } else {
            None
        }
    }

    fn draw_params(&self) -> DrawParameters<'static> {
        let mut params = DrawParameters {
            depth: Depth {
                test: DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: BackfaceCullingMode::CullCounterClockwise,
            blend: Blend::alpha_blending(),
            .. Default::default()
        };

        if self.debug {
            params.polygon_mode = PolygonMode::Line;
        }

        params
    }

    pub fn render_3d_lines<I: Iterator<Item=Vector3<f32>>>(&mut self, iterator: I) {
        let mut last = None;

        for vector in iterator {
            let vertex = Vertex::new(vector);

            if let Some(last) = last {
                self.lines_3d.push(last);
                self.lines_3d.push(vertex);
            }

            last = Some(vertex);
        }
    }

    // http://webglfactory.blogspot.com/2011/05/how-to-convert-world-to-screen.html
    pub fn ray(&self, camera: &Camera, mouse: (f32, f32)) -> collision::Ray<f32, Point3<f32>, Vector3<f32>> {
        // Get mouse position
        let (x, y) = mouse;

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

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
    }

    pub fn render_image(&mut self, image: Image, x: f32, y: f32, width: f32, height: f32, overlay: [f32; 4]) {
        self.lines.render_image(image, x, y, width, height, overlay, &mut self.target, &self.display, &self.resources)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        self.target.set_finish().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use glium::*;
    use glutin::*;

    #[test]
    fn test_shader() {
        let context = HeadlessRendererBuilder::new(640, 480).build().unwrap();
        let display = HeadlessRenderer::new(context).unwrap();
        // Try create the program
        Program::from_source(&display, super::VERT, super::FRAG, None)
            .unwrap_or_else(|error| panic!("\n{}", error));
    }

    #[test]
    fn test_lines_shader() {
        let context = HeadlessRendererBuilder::new(640, 480).build().unwrap();
        let display = HeadlessRenderer::new(context).unwrap();
        // Try create the program
        Program::from_source(&display, super::lines::VERT, super::lines::FRAG, None)
            .unwrap_or_else(|error| panic!("\n{}", error));
    }
}