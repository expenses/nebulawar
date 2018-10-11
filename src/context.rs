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
use lines::*;
use *;

use runic;

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
            text_program: runic::default_program(&display).unwrap(),
            display
        }
    }

    pub fn clear(&mut self) {
        self.target.clear_color_and_depth((0.0, 0.5, 0.0, 1.0), 1.0);
    }

    pub fn finish(&mut self) {
        self.target.set_finish().unwrap();
        self.target = self.display.draw();
    }

    fn uniforms<'a>(&self, position: Matrix4<f32>, camera: &Camera, light: [f32; 3], texture: &'a SrgbTexture2d, shadeless: bool) -> impl Uniforms + 'a {
        uniform!{
            model: matrix_to_array(position),
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: light,
            tex: Sampler::new(texture).minify_filter(MinifySamplerFilter::Nearest).magnify_filter(MagnifySamplerFilter::Nearest),
            shadeless: shadeless
        }
    }

    pub fn render(&mut self, model: usize, position: Matrix4<f32>, camera: &Camera, light: [f32; 3]) {
        let model = &self.resources.models[model];

        let uniforms = self.uniforms(position, camera, light, &model.texture, false);

        let params = DrawParameters {
            depth: Depth {
                test: DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: BackfaceCullingMode::CullCounterClockwise,
            .. Default::default()
        };

        self.target.draw(&model.vertices, &NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();
    }

    pub fn render_skybox(&mut self, camera: &Camera, index: usize) {
        let skybox_position = Matrix4::from_translation(camera.position()) * Matrix4::from_scale(100.0);

        let uniforms = self.uniforms(skybox_position, camera, [0.0; 3], &self.resources.skybox_images[index], true);

        let params = DrawParameters {
            depth: Depth {
                test: DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: BackfaceCullingMode::CullCounterClockwise,
            .. Default::default()
        };

        self.target.draw(&self.resources.skybox, &NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();
    }

    pub fn render_text(&mut self, text: &str, x: f32, y: f32) {
        self.resources.font.render(text, [x, y], 20.0, [1.0; 4], &mut self.target, &self.display, &self.text_program).unwrap();
    }

    pub fn render_line(&mut self, start: (f32, f32), end: (f32, f32)) {
        self.lines.render_line(start, end, &mut self.target, &self.display);
    }

    pub fn render_circle(&mut self, x: f32, y: f32, radius: f32) {
        self.lines.render_circle(x, y, radius, &mut self.target, &self.display);
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

    pub fn screen_position(&self, model: Matrix4<f32>, camera: &Camera) -> Option<(f32, f32)> {
        let modelview = camera.view_matrix() * model;

        let gl_position = self.perspective_matrix() * modelview * Vector4::new(0.0, 0.0, 0.0, 1.0);

        let x = gl_position[0] / gl_position[3];
        let y = gl_position[1] / gl_position[3];
        let z = gl_position[2] / gl_position[3];

        if z < 1.0 {
            let (width, height) = self.screen_dimensions();
            let (x, y) = opengl_pos_to_screen_pos(x, y, width, height);
            Some((x * 2.0 / self.dpi(), y * 2.0 / self.dpi()))
        } else {
            None
        }
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