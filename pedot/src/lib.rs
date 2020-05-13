extern crate glium;
#[macro_use]
extern crate derive_is_enum_variant;

use glium::*;
use glutin::*;
use glutin::event::*;
use glutin::dpi::*;

use std::ops::*;
use std::slice::*;

mod alignment;
pub use alignment::*;




#[derive(PartialEq, Debug, is_enum_variant)]
pub enum ButtonState {
    Clicked(f32, f32),
    Hovering(f32, f32),
    None
}

#[derive(Debug)]
pub struct Gui {
    screen_width: f32,
    screen_height: f32,
    last_char: Option<char>,
    last_keypress: Option<VirtualKeyCode>,
    mouse_x: f32,
    mouse_y: f32,
    mouse_clicked: bool
}

impl Gui {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width, screen_height,
            last_char: None,
            last_keypress: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_clicked: false
        }
    }

    pub fn clear(&mut self) {
        self.last_char = None;
        self.last_keypress = None;
    }

    pub fn update(&mut self, event: &WindowEvent) {
        match *event {
            WindowEvent::Resized(PhysicalSize {width, height}) => {
                self.screen_width = width as f32;
                self.screen_height = height as f32;
            },
            WindowEvent::CursorMoved {position: PhysicalPosition {x, y}, ..} => {
                self.mouse_x = x as f32;
                self.mouse_y = y as f32;
            },
            WindowEvent::MouseInput {button: MouseButton::Left, state, ..} => self.mouse_clicked = state == ElementState::Pressed,
            WindowEvent::ReceivedCharacter(character) => self.last_char = Some(character),
            WindowEvent::KeyboardInput {input: KeyboardInput {virtual_keycode: Some(key), state: ElementState::Pressed, ..}, ..} => self.last_keypress = Some(key),
            _ => {}
        }
    }

    pub fn button<X: Into<HorizontalAlign>, Y: Into<VerticalAlign>>(&self, x: X, y: Y, width: f32, height: f32) -> ButtonState {
        let x = x.into();
        let y = y.into();

        let hovering =
            self.mouse_x >= self.x_absolute(x) - width  / 2.0 &&
            self.mouse_y >= self.y_absolute(y) - height / 2.0 &&
            self.mouse_x <= self.x_absolute(x) + width  / 2.0 &&
            self.mouse_y <= self.y_absolute(y) + height / 2.0;

        match (hovering, self.mouse_clicked) {
            (true, true) => ButtonState::Clicked(self.mouse_x, self.mouse_y),
            (true, false) => ButtonState::Hovering(self.mouse_x, self.mouse_y),
            _ => ButtonState::None
        }
    }

    pub fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.last_keypress == Some(key)
    }

    pub fn key_input<P: Fn(char) -> bool>(&self, text: &mut String, predicate: P) -> bool {
        if let Some(character) = self.last_char.filter(|character| predicate(*character)) {
            text.push(character);
            true
        } else {
            false
        }
    }

    pub fn x_absolute(&self, x: HorizontalAlign) -> f32 {
        x.absolute(self.screen_width)
    }

    pub fn y_absolute(&self, y: VerticalAlign) -> f32 {
        y.absolute(self.screen_height)
    }
}

pub struct List<T> {
    entries: Vec<T>,
    index: usize
}

impl<T> List<T> {
    pub fn new(entries: Vec<T>) -> Self {
        Self {
            entries,
            index: 0
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn set_index(&mut self, index: usize) {
        assert!(index < self.entries.len());
        self.index = index;
    }

    pub fn set_entries(&mut self, entries: Vec<T>) {
        self.entries = entries;
        self.index = self.index.min(self.entries.len() - 1);
    }

    pub fn clear_entries(&mut self) {
        self.index = 0;
        self.entries.clear();
    }

    pub fn push_entry(&mut self, entry: T) {
        self.entries.push(entry);
    }

    pub fn insert_entry(&mut self, index: usize, entry: T) {
        self.entries.insert(index, entry);
    }

    pub fn rotate_up(&mut self) {
        self.index = self.index.checked_sub(1).unwrap_or(self.entries.len() - 1);
    }

    pub fn rotate_down(&mut self) {
        self.index = (self.index + 1) % self.entries.len();
    }

    pub fn get(&self) -> &T {
        &self.entries[self.index]
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.entries[self.index]
    }

    pub fn iter(&self) -> Iter<T> {
        self.entries.iter()
    }
}

impl<T> Index<usize> for List<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.entries[index]
    }
}

impl<T> IndexMut<usize> for List<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.entries[index]
    }
}
