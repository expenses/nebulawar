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

    // Use the pixelated program for rendering fonts
    let program = runic::default_program(&display).unwrap();
    // Create a cached font
    let mut cached_font = runic::CachedFont::from_bytes(include_bytes!("fonts/TinyUnicode.ttf"), &display).unwrap();

    let mut text: String = "Try typing: ".into();

    let mut running = true;
    let mut scale = 2.0;

    while running {
        events_loop.poll_events(|event| if let Event::WindowEvent {event, ..} = event {
            match event {
                WindowEvent::CloseRequested => running = false,
                // Push characters to the text
                WindowEvent::ReceivedCharacter(character) if character.is_ascii() => text.push(character),
                WindowEvent::KeyboardInput {input: KeyboardInput {virtual_keycode: Some(key), state: ElementState::Pressed, ..}, ..} => {
                    match key {
                        VirtualKeyCode::Up => scale += 1.0,
                        VirtualKeyCode::Down => scale -= 1.0,
                        _ => {}
                    }
                },
                _ => {}
            }
        });

        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        // The font has a base size of 13, but we want to scale that up
        cached_font.render_pixelated(&text, [33.3, 33.3], 13.0, scale, [0.0, 0.0, 0.0, 1.0], &mut target, &display, &program).unwrap();
        target.finish().unwrap();
    }
}
