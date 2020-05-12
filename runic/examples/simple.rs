extern crate glium;
extern crate runic;

use glium::*;
use glutin::*;

fn main() {
    // Create the events loop and display as per normal
    let mut events_loop = EventsLoop::new();

    let window = glutin::WindowBuilder::new()
        .with_dimensions((512, 512).into())
        .with_title("Runic Simple Example");

    let context = glutin::ContextBuilder::new()
        .with_vsync(true);
    
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // Get the default program for rendering fonts
    let program = runic::default_program(&display).unwrap();
    // Create a cached font
    let mut cached_font = runic::CachedFont::from_bytes(include_bytes!("fonts/WenQuanYiMicroHei.ttf"), &display).unwrap();

    let mut text: String = "Try typing: ".into();

    let mut running = true;
    while running {
        events_loop.poll_events(|event| if let Event::WindowEvent {event, ..} = event {
            match event {
                WindowEvent::CloseRequested => running = false,
                // Push characters to the text
                WindowEvent::ReceivedCharacter(character) => text.push(character),
                _ => {}
            }
        });

        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        // Render the font at (10, 10), with a font size of 24 and a black colour
        cached_font.render(&text, [10.0, 10.0], 24.0, [0.0, 0.0, 0.0, 1.0], &mut target, &display, &program).unwrap();
        target.finish().unwrap();
    }
}