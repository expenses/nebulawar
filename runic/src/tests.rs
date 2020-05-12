#![cfg(test)]

use *;

// Comparing floats with a relative allowed gap
fn close_enough(a: f32, b: f32) -> bool {
    println!("Comparing '{}' and '{}'", a, b);
    (a - b).abs() <= 1.0e-06 * a.abs().max(b.abs())
}

#[test]
fn test_position() {
    assert_eq!(screen_pos_to_opengl_pos([500.0, 500.0], 1000.0, 1000.0), [0.0, 0.0]);
    assert_eq!(screen_pos_to_opengl_pos([0.0, 0.0], 1000.0, 1000.0), [-1.0, 1.0]);
    assert_eq!(screen_pos_to_opengl_pos([1000.0, 0.0], 1000.0, 1000.0), [1.0, 1.0]);
    assert_eq!(screen_pos_to_opengl_pos([0.0, 1000.0], 1000.0, 1000.0), [-1.0, -1.0]);
    assert_eq!(screen_pos_to_opengl_pos([1000.0, 1000.0], 1000.0, 1000.0), [1.0, -1.0]);
    assert_eq!(screen_pos_to_opengl_pos([750.0, 250.0], 1000.0, 1000.0), [0.5, 0.5]);   
}

#[test]
fn compile_shaders() {
    use glium::glutin::*;
    use glium::glutin::dpi::*;
    use glium::backend::glutin::headless::*;
    
    let context_builder = ContextBuilder::new();
    let event_loop = EventsLoop::new();
    let context = context_builder.build_headless(&event_loop, PhysicalSize::new(1000.0, 1000.0)).unwrap();
    
    let display = Headless::new(context).unwrap();
    default_program(&display).unwrap();
}

#[test]
fn test_vertex_gen() {
    use glium::glutin::*;
    use glium::glutin::dpi::*;
    use glium::backend::glutin::headless::*;
    
    let context_builder = ContextBuilder::new();
    let event_loop = EventsLoop::new();
    let context = context_builder.build_headless(&event_loop, PhysicalSize::new(1000.0, 1000.0)).unwrap();

    let display = Headless::new(context).unwrap();    
    let mut font = CachedFont::from_bytes(include_bytes!("../examples/fonts/WenQuanYiMicroHei.ttf"), &display).unwrap();

    // The HeadlessRendererBuilder dimensions != the framebuffer dimensions for whatever reason
    let (width, height) = display.framebuffer_dimensions();

    let origin_a = [0.0, 0.0];
    let origin_b = [100.0, 666.6];

    let vertices_a: Vec<_> = font.get_vertices("Hello World", origin_a, 32.0, false, &display).unwrap().collect();
    let vertices_b: Vec<_> = font.get_vertices("Hello World", origin_b, 32.0, false, &display).unwrap().collect();

    let vertices_a_pixelated: Vec<_> = font.get_vertices("Hello World", origin_a, 32.0, true, &display).unwrap().collect();
    let vertices_b_pixelated: Vec<_> = font.get_vertices("Hello World", origin_b, 32.0, true, &display).unwrap().collect();
    
    // Get the difference in opengl coordinates between the two origins

    let origin_a = screen_pos_to_opengl_pos(origin_a, width as f32, height as f32);
    let origin_b = screen_pos_to_opengl_pos(origin_b, width as f32, height as f32);

    let delta = [
        origin_b[0] - origin_a[0],
        origin_b[1] - origin_a[1]
    ];

    // assert that the vertices plus the delta match, meaning that they are offset correctly
    
    for (a, b) in vertices_a.iter().zip(vertices_b.iter()) {
        println!("#####\n{:?}\n{:?}", a, b);
        assert!(close_enough(a.in_pos[0] + delta[0], b.in_pos[0]));
        assert!(close_enough(a.in_pos[1] + delta[1], b.in_pos[1]));
    }

    for (a, b) in vertices_a_pixelated.iter().zip(vertices_b_pixelated.iter()) {
        println!("#####\n{:?}\n{:?}", a, b);
        assert!(close_enough(a.in_pos[0] + delta[0], b.in_pos[0]));
        assert!(close_enough(a.in_pos[1] + delta[1], b.in_pos[1]));
    }
}
