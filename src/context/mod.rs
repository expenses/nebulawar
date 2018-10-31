mod lines;
mod resources;

use self::lines::*;

pub use self::resources::*;
pub const WHITE: [f32; 3] = [1.0; 3];

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
use collision::primitive::ConvexPolyhedron;

// ** Line Rendering Woes **
// rendering in 2d: doesnt work with rest of scene, rendering lines that go behind the camera is hard
// gl_lines: has a max width of 1 on my laptop
// 2d lines in 3d: getting lines to join nicely is hard, too flat
// geometry shader: complicated
// assembling triangle/square line meshes by hand: complicated, but might be best shot

const VERT: &str = include_str!("shaders/shader.vert");
const FRAG: &str = include_str!("shaders/shader.frag");

const DEFAULT_WIDTH: f32 = 1280.0;
const DEFAULT_HEIGHT: f32 = 800.0;

pub enum Mode {
    Normal = 1,
    Shadeless = 2,
    White = 3,
    VertexColoured = 4
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture: [f32; 2],
}

impl Vertex {
    pub fn with_brightness(position: Vector3<f32>, brightness: f32) -> Self {
        Self {
            position: position.into(),
            normal: [0.0; 3],
            texture: [brightness; 2]
        }
    }

    pub fn with_colour(position: Vector3<f32>, colour: [f32; 3]) -> Self {
        Self {
            position: position.into(),
            normal: colour,
            texture: [0.0; 2]
        }
    }
}

implement_vertex!(Vertex, position, normal, texture);

pub struct Context {
    display: Display,
    program: Program,
    target: Frame,
    resources: Resources,
    lines: LineRenderer,
    text_program: Program,
    
    text_buffer: Vec<runic::Vertex>,
    lines_3d_buffer: Vec<Vertex>,
    
    smoke_buffer: Vec<Vertex>,

    debug: bool,
    pub gui: Gui
}

impl Context {
    pub fn new(events_loop: &EventsLoop) -> Self {
        let window = glutin::WindowBuilder::new()
            .with_dimensions(LogicalSize::new(DEFAULT_WIDTH.into(), DEFAULT_HEIGHT.into()))
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
            resources: Resources::new(&display).unwrap(),
            target: display.draw(),
            lines: LineRenderer::new(&display),
            text_program: runic::default_program(&display).unwrap(),
            display, program,
            
            text_buffer: Vec::new(),
            lines_3d_buffer: Vec::new(),
            smoke_buffer: Vec::new(),
            
            debug: false,
            gui: Gui::new(DEFAULT_WIDTH, DEFAULT_HEIGHT)
        }
    }

    pub fn copy_event(&mut self, event: &WindowEvent) {
        self.gui.update(event);
    }

    pub fn clear(&mut self) {
        self.target.clear_color_srgb_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    }

    fn flush_3d_lines(&mut self, camera: &Camera, system: &StarSystem) {
        let uniforms = uniform!{
            model: matrix_to_array(Matrix4::identity()),
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            mode: Mode::VertexColoured as i32
        };

        let vertices = VertexBuffer::new(&self.display, &self.lines_3d_buffer).unwrap();
        let indices = NoIndices(PrimitiveType::LinesList);

        let params = self.draw_params();
        self.target.draw(&vertices, &indices, &self.program, &uniforms, &params).unwrap();

        self.lines_3d_buffer.clear();
    }

    fn flush_text(&mut self) {
        self.resources.font.render_vertices(&self.text_buffer, [1.0; 4], &mut self.target, &self.display, &self.text_program, true).unwrap();
        self.text_buffer.clear();
    }

    pub fn flush_ui(&mut self, camera: &Camera, system: &StarSystem) {
        self.lines.flush(&mut self.target, &self.display);

        self.flush_3d_lines(camera, system);
        self.flush_text();

        self.gui.clear();
    }

    pub fn finish(&mut self) {
        self.target.set_finish().unwrap();
        self.target = self.display.draw();
    }

    pub fn render_billboard(&mut self, matrix: Matrix4<f32>, image: Image, camera: &Camera, system: &StarSystem) {
        let uniforms = self.uniforms(matrix, camera, system, &self.resources.image, Mode::Shadeless);
        let params = self.draw_params();

        let mut vertices = BILLBOARD_VERTICES;

        for mut vertex in vertices.iter_mut() {
            vertex.texture = image.translate(vertex.texture);
        }

        self.target.draw(
            &VertexBuffer::new(&self.display, &vertices).unwrap(),
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &params
        ).unwrap();
    }

    pub fn render_stars(&mut self, system: &StarSystem, camera: &Camera) {
        let uniforms = self.background_uniforms(camera, system, Mode::White);

        let vertices = VertexBuffer::new(&self.display, &system.stars).unwrap();
        let indices = NoIndices(PrimitiveType::Points);

        let params = DrawParameters {
            polygon_mode: PolygonMode::Point,
            point_size: Some(2.0 * self.dpi()),
            .. Self::draw_params(self)
        };

        self.target.draw(&vertices, &indices, &self.program, &uniforms, &params).unwrap();
    }

    pub fn render_skybox(&mut self, system: &StarSystem, camera: &Camera) {
        let uniforms = self.background_uniforms(camera, system, Mode::VertexColoured);

        let vertices = VertexBuffer::new(&self.display, &system.background).unwrap();
        let indices = NoIndices(PrimitiveType::TrianglesList);

        let params = self.draw_params();

        self.target.draw(&vertices, &indices, &self.program, &uniforms, &params).unwrap();
    }

    pub fn flush_smoke(&mut self, system: &StarSystem, camera: &Camera) {
        let uniforms = self.uniforms(Matrix4::identity(), camera, system, &self.resources.image, Mode::Shadeless);
        let params = DrawParameters {
            depth: Depth {
                test: DepthTest::IfLess,
                .. Default::default()
            },
            .. Self::draw_params(self)
        };

        let buffer = VertexBuffer::new(&self.display, &self.smoke_buffer).unwrap();

        self.target.draw(
            &buffer,
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &params
        ).unwrap();

        self.smoke_buffer.clear();
    }

    pub fn render_smoke<I: Iterator<Item=Vertex>>(&mut self, iterator: I, len: usize) {
        if len > self.smoke_buffer.len() {
            let difference = len - self.smoke_buffer.len();
            self.smoke_buffer.reserve(difference);
        }

        for vertex in iterator {
            self.smoke_buffer.push(vertex);
        }
    }

    pub fn background_uniforms<'a>(&self, camera: &Camera, system: &StarSystem, mode: Mode) -> impl Uniforms + 'a {
        uniform! {
            model: matrix_to_array(Matrix4::identity()),
            view: matrix_to_array(camera.view_matrix_only_direction()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            mode: mode as i32
        }
    }

    pub fn uniforms<'a>(&self, position: Matrix4<f32>, camera: &Camera, system: &StarSystem, texture: &'a SrgbTexture2d, mode: Mode) -> impl Uniforms + 'a {
        uniform!{
            model: matrix_to_array(position),
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            tex: Sampler::new(texture).minify_filter(MinifySamplerFilter::Nearest).magnify_filter(MagnifySamplerFilter::Nearest),
            mode: mode as i32
        }
    }

    pub fn render_model(&mut self, model: Model, location: Vector3<f32>, rotation: Quaternion<f32>, size: f32, camera: &Camera, system: &StarSystem) {
        let scale = Matrix4::from_scale(size);
        let rotation: Matrix4<f32> = rotation.into();
        let position = Matrix4::from_translation(location) * rotation * scale;

        let model = &self.resources.models[model as usize];

        let uniforms = self.uniforms(position, camera, system, &model.texture, Mode::Normal);
        let params = self.draw_params();

        self.target.draw(&model.vertices, &NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();
    }

    pub fn render_text(&mut self, text: &str, x: f32, y: f32) {
        let iterator = self.resources.font.get_pixelated_vertices(text, [x, y], 16.0, 1.0, &self.display).unwrap();
        self.text_buffer.extend(iterator);
    }

    pub fn render_rect(&mut self, top_left: (f32, f32), bottom_right: (f32, f32)) {
        self.lines.render_rect(top_left, bottom_right);
    }

    pub fn render_circle(&mut self, x: f32, y: f32, radius: f32, colour: [f32; 4]) {
        self.lines.render_circle(x, y, radius, colour);
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

    // todo: move screen dims into systems so the pers matrix can be generated

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

    pub fn render_3d_lines<I: Iterator<Item=Vector3<f32>>>(&mut self, iterator: I, colour: [f32; 3]) {
        let mut last = None;

        for vector in iterator {
            let vertex = Vertex::with_colour(vector, colour);

            if let Some(last) = last {
                self.lines_3d_buffer.push(last);
                self.lines_3d_buffer.push(vertex);
            }

            last = Some(vertex);
        }
    }

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
    }

    pub fn is_debugging(&self) -> bool {
        self.debug
    }

    pub fn render_image(&mut self, image: Image, x: f32, y: f32, width: f32, height: f32, overlay: [f32; 4]) {
        self.lines.render_image(image, x, y, width, height, overlay, &mut self.target, &self.display, &self.resources)
    }

    pub fn collision_mesh(&self, model: Model) -> &ConvexPolyhedron<f32> {
        &self.resources.models[model as usize].collision_mesh
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
    #[cfg(not(target_os = "linux"))]
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