#[macro_use]
extern crate glium;
extern crate obj;
extern crate genmesh;
extern crate image;
extern crate line_drawing;
extern crate arrayvec;
extern crate cgmath;

use glium::*;
use glium::uniforms::*;
use glium::texture::*;
use std::fs::*;
use std::io::*;
use glutin::*;
use glutin::dpi::*;

mod camera;
mod lines;

use camera::*;
use lines::*;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    texture: [f32; 2],
}

implement_vertex!(Vertex, position, normal, texture);

#[derive(Default)]
struct Controls {
    mouse: (f64, f64),
    left_pressed: bool,
    left_drag: Option<(f64, f64)>,
    right_pressed: bool,
    left: bool,
    right: bool,
    forwards: bool,
    back: bool
}

impl Controls {
    fn right_dragging(&self) -> bool {
        self.mouse != (0.0, 0.0) && self.right_pressed
    }
}

struct World {
    light: [f32; 3]
}

impl World {
    fn new() -> Self {
        Self {
            light: [50.0, 50.0, 50.0]
        }
    }
}

struct Object {
    model: usize,
    transform: [[f32; 4]; 4]
}



struct Renderer {
    display: Display,
    program: Program,
    target: Frame,
    models: [Model; 2],
    world: World,
    controls: Controls,
    lines: LineRenderer
}

#[derive(Default)]
struct Game {
    camera: Camera,
    objects: Vec<Object>
}

impl Renderer {
    fn new(events_loop: &EventsLoop) -> Self {
        let window = glutin::WindowBuilder::new();
        let context = glutin::ContextBuilder::new()
            .with_multisampling(4)
            .with_depth_buffer(24)
            .with_vsync(true);
        
        let display = glium::Display::new(window, context, &events_loop).unwrap();

        let fighter = Model::new(&display, "fighter.obj", "fighter.png");
        let skybox = Model::new(&display, "skybox.obj", "skybox.png");

        let program = glium::Program::from_source(
                &display,
                include_str!("shader.vert"),
                include_str!("shader.frag"),
                None
        ).unwrap();

        Self {
            target: display.draw(),
            program: program,
            models: [fighter, skybox],
            lines: LineRenderer::new(&display),
            display,
            world: World::new(),
            controls: Controls::default()
        }
    }

    fn clear(&mut self) {
        self.target.clear_color_and_depth((0.0, 0.5, 0.0, 1.0), 1.0);
    }

    fn finish(&mut self) {
        self.target.set_finish().unwrap();
        self.target = self.display.draw();
    }

    fn render(&mut self, model: usize, position: [[f32; 4]; 4], camera: &Camera, shadeless: bool) {
        let (width, height) = self.target.get_dimensions();
        let perspective = perspective_matrix(width as f32, height as f32);

        let view = camera.view_matrix();

        let light = self.world.light;

        let uniforms = uniform!{
            model: position, view: view, perspective: perspective, light_direction: light,
            tex: Sampler::new(&self.models[model].texture).minify_filter(MinifySamplerFilter::Nearest).magnify_filter(MagnifySamplerFilter::Nearest),
            shadeless: shadeless
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
            .. Default::default()
        };

        self.target.draw(&self.models[model].vertices, &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();
    }

    fn render_skybox(&mut self, camera: &Camera) {
        let camera_position = camera.position();

        let skybox_position = [
            [100.0, 0.0, 0.0, 0.0],
            [0.0, 100.0, 0.0, 0.0],
            [0.0, 0.0, 100.0, 0.0],
            [camera_position[0], camera_position[1], camera_position[2], 1.0_f32]
        ];

        self.render(1, skybox_position, camera, true);
    }

    fn move_mouse(&mut self, x: f64, y: f64) -> Option<(f64, f64)> {
        let delta = if self.controls.right_dragging() {
            let (mouse_x, mouse_y) = self.controls.mouse;
            Some((x - mouse_x, y - mouse_y))
        } else {
            None
        };

        self.controls.mouse = (x, y);

        delta
    }

    fn render_line(&mut self, start: (i32, i32), end: (i32, i32)) {
        self.lines.render_line(start, end, &mut self.target, &self.display);
    }

    fn render_circle(&mut self, center: (i32, i32), radius: i32) {
        self.lines.render_circle(center, radius, &mut self.target, &self.display);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.target.set_finish().unwrap();
    }
}


fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    
    let mut renderer = Renderer::new(&events_loop);

    let mut camera = Camera::default();

    let mut ship = Object {
        model: 0,
        transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]
    };

    let mut time: f32 = 0.0;

    let mut closed = false;
    while !closed {
        events_loop.poll_events(|event| if let glutin::Event::WindowEvent {event, ..} = event {
            match event {
                glutin::WindowEvent::CloseRequested => closed = true,
                glutin::WindowEvent::CursorMoved {position: LogicalPosition {x, y}, ..} => {
                    let (mouse_x, mouse_y) = renderer.controls.mouse;
                    let (delta_x, delta_y) = (x - mouse_x, y - mouse_y);

                    if renderer.controls.right_dragging() {
                        camera.rotate_longitude(delta_x as f32 / 200.0);
                        camera.rotate_latitude(delta_y as f32 / 200.0);
                    } else if renderer.controls.left_pressed && renderer.controls.left_drag.is_none() {
                        renderer.controls.left_drag = Some((x, y));
                    }

                    renderer.controls.mouse = (x, y);
                },
                glutin::WindowEvent::MouseInput {state, button, ..} => {
                    let pressed = state == ElementState::Pressed;

                    match button {
                        MouseButton::Left => {
                            renderer.controls.left_pressed = pressed;
                            renderer.controls.left_drag.take();
                        }
                        MouseButton::Right => renderer.controls.right_pressed = pressed,
                        _ => {}
                    }
                },
                glutin::WindowEvent::KeyboardInput {input: KeyboardInput {state, virtual_keycode: Some(key), ..}, ..} => {
                    let pressed = state == ElementState::Pressed;
                    
                    match key {
                        VirtualKeyCode::Left => renderer.controls.left = pressed,
                        VirtualKeyCode::Right => renderer.controls.right = pressed,
                        VirtualKeyCode::Up => renderer.controls.forwards = pressed,
                        VirtualKeyCode::Down => renderer.controls.back = pressed,
                        _ => {}
                    }
                },
                glutin::WindowEvent::MouseWheel {delta, ..} => match delta {
                    MouseScrollDelta::PixelDelta(LogicalPosition {x: _, y}) => camera.change_distance(y as f32 / 20.0),
                    MouseScrollDelta::LineDelta(_, _) => unimplemented!("Line delta not implemented yet")
                },
                _ => ()
            }
        });

        ship.transform = [
            [time.sin(), 0.0, time.cos(), 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-time.cos(), 0.0, time.sin(), 0.0],
            [time.sin() * 10.0, 0.0, time.cos() * 10.0, 1.0]
        ];

        if renderer.controls.left {
            camera.move_sideways(-0.2);
        }

        if renderer.controls.right {
            camera.move_sideways(0.2);
        }

        if renderer.controls.forwards {
            camera.move_forwards(0.2);
        }

        if renderer.controls.back {
            camera.move_forwards(-0.2);
        }

        renderer.clear();

        renderer.render(ship.model, ship.transform, &camera, false);

        renderer.render_skybox(&camera);

        let (width, height) = renderer.target.get_dimensions();
        let perspective: cgmath::Matrix4<f32> = perspective_matrix(width as f32, height as f32).into();

        let view: cgmath::Matrix4<f32> = camera.view_matrix().into();
        let model: cgmath::Matrix4<f32> = ship.transform.into();

        let modelview: cgmath::Matrix4<f32> = view * model;

        let result = perspective * modelview * cgmath::Vector4::new(0.0, 0.0, 0.0, 1.0);

        let x = result[0] / result[3];
        let y = result[1] / result[3];

        let screen_x = (x + 1.0) / 2.0 * width as f32 / 2.0;
        let screen_y = (1.0 - y) / 2.0 * height as f32 / 2.0;

        renderer.render_circle((screen_x as i32, screen_y as i32), 50);

        println!("{} {}", x, y);


        if let Some((start_x, start_y)) = renderer.controls.left_drag {
            let (end_x, end_y) = renderer.controls.mouse;
            let (start_x, start_y) = (start_x as i32, start_y as i32);
            let (end_x, end_y) = (end_x as i32, end_y as i32);

            renderer.render_line((start_x, start_y), (end_x, start_y));
            renderer.render_line((start_x, end_y), (end_x, end_y));

            renderer.render_line((start_x, start_y), (start_x, end_y));
            renderer.render_line((end_x, start_y), (end_x, end_y));
        }
        renderer.finish();

        time += 0.02;
    }
}

struct Model {
    vertices: VertexBuffer<Vertex>,
    texture: SrgbTexture2d
}

impl Model {
    fn new(display: &Display, model: &str, image_filename: &str) -> Self {
        let image = image::open(image_filename).unwrap().to_rgba();
        let image_dimensions = image.dimensions();
        let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        let texture = SrgbTexture2d::new(display, image).unwrap();

        let mut buffer = Vec::new();
        File::open(model).unwrap().read_to_end(&mut buffer).unwrap();

        Self {
            vertices: load_wavefront(display, &buffer),
            texture: texture
        }
    }
}


/// Returns a vertex buffer that should be rendered as `TrianglesList`.
fn load_wavefront(display: &glium::Display, data: &[u8]) -> glium::VertexBuffer<Vertex> {
    let mut data = BufReader::new(data);
    let data = obj::Obj::load_buf(&mut data).unwrap();

    let mut vertex_data = Vec::new();

    for object in data.objects.iter() {
        for polygon in object.groups.iter().flat_map(|g| g.polys.iter()) {
            match &polygon {
                genmesh::Polygon::PolyTri(genmesh::Triangle { x: v1, y: v2, z: v3 }) => {
                    for v in [v1, v2, v3].iter() {
                        let position = data.position[v.0];
                        let texture = v.1.map(|index| data.texture[index]);
                        let normal = v.2.map(|index| data.normal[index]);

                        let texture = texture.unwrap_or([0.0, 0.0]);
                        let normal = normal.unwrap_or([0.0, 0.0, 0.0]);

                        vertex_data.push(Vertex {
                            position: position,
                            normal: normal,
                            texture: texture,
                        })
                    }
                },
                genmesh::Polygon::PolyQuad(_) => unimplemented!("Quad polygons not supported, use triangles instead.")
            }
        }
    }

    VertexBuffer::new(display, &vertex_data).unwrap()
}

fn perspective_matrix(width: f32, height: f32) -> [[f32; 4]; 4] {
    let aspect_ratio = height as f32 / width as f32;

    let fov: f32 = 3.141592 / 3.0;
    let zfar = 10240.0;
    let znear = 0.1;

    let f = 1.0 / (fov / 2.0).tan();

    [
        [f *   aspect_ratio   ,    0.0,              0.0              ,   0.0],
        [         0.0         ,     f ,              0.0              ,   0.0],
        [         0.0         ,    0.0,  (zfar+znear)/(zfar-znear)    ,   1.0],
        [         0.0         ,    0.0, -(2.0*zfar*znear)/(zfar-znear),   0.0],
    ]
}