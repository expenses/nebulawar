extern crate glium;
extern crate runic;

const FRAG: &str = include_str!("fancy.frag");

use glium::*;
use glutin::*;
use glium::uniforms::*;

fn main() {
    // Create the events loop, and display as per normal
    let mut events_loop = EventsLoop::new();

    let window = glutin::WindowBuilder::new()
        .with_dimensions((512, 512).into())
        .with_title("Runic Fancy Example");

    let context = glutin::ContextBuilder::new()
        .with_vsync(true);
    
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // Create a custom program with our own fragmentation shader
    let program = Program::from_source(&display, runic::VERT, FRAG, None).unwrap();
    // Create a cached font
    let mut cached_font = runic::CachedFont::from_bytes(include_bytes!("fonts/WenQuanYiMicroHei.ttf"), &display).unwrap();

    let mut text: String = "It does custom shaders too: ".into();

    let mut running = true;
    let mut time = 0.0_f32;

    while running {
        events_loop.poll_events(|event| if let Event::WindowEvent {event, ..} = event {
            match event {
                WindowEvent::CloseRequested => running = false,
                WindowEvent::ReceivedCharacter(character) => text.push(character),
                _ => {}
            }
        });

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        // Get the vertices for the font and make a vertex buffer
        let vertices: Vec<_> = cached_font.get_vertices(&text, [10.0, 10.0], 24.0, false, &display).unwrap().collect();
        let vertex_buffer = VertexBuffer::new(&display, &vertices).unwrap();

        // Setup uniforms
        let uniforms = uniform! {
            sampler: Sampler::new(cached_font.cache().texture()),
            colour: [1.0, 0.0, 0.0, 1.0_f32],
            time: time
        };

        // Draw the font to the frame!
        target.draw(
            &vertex_buffer,
            glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            &program,
            &uniforms,
            &glium::DrawParameters {
                blend: glium::Blend::alpha_blending(),
                ..Default::default()
            }
        ).unwrap();

        target.finish().unwrap();

        time += 0.1;
    }
}