// up -> down -> clicked | dragging
// clicked -> up
// dragging -> dragged -> up

use std::mem::swap;
use specs::*;

#[derive(is_enum_variant, Debug, Clone)]
enum MouseState {
    Dragging(f32, f32),
    Dragged(f32, f32),
    Up,
    Clicked,
    Down(u8, f32, f32)
}

impl MouseState {
    fn update(&mut self, mouse: (f32, f32)) {
        let (mouse_x, mouse_y) = mouse;
        match *self {
            MouseState::Clicked => *self = MouseState::Up,
            MouseState::Down(frames, x, y) if frames > 10 || (mouse_x - x).abs() > 10.0 || (mouse_y - y).abs() > 10.0 => *self = MouseState::Dragging(x, y),
            MouseState::Down(ref mut frames, _, _) => *frames += 1,
            MouseState::Dragged(_, _) => *self = MouseState::Up,
            _ => {}
        }
    }

    fn handle(&mut self, mouse: (f32, f32), pressed: bool) {
        if pressed {
            self.handle_down(mouse);
        } else {
            self.handle_up();
        }
    }

    fn handle_down(&mut self, mouse: (f32, f32)) {
        let (x, y) = mouse;
        *self = MouseState::Down(0, x, y)
    }

    fn handle_up(&mut self) {
        match *self {
            MouseState::Down(_, _, _) => *self = MouseState::Clicked,
            MouseState::Dragging(x, y) => *self = MouseState::Dragged(x, y),
            _ => *self = MouseState::Up
        }
    }
}

impl Default for MouseState {
    fn default() -> Self {
        MouseState::Up
    }
}

#[derive(Default, Component, Clone)]
pub struct Controls {
    mouse: (f32, f32),

    left_state: MouseState,
    right_state: MouseState,
    middle_state: MouseState,

    pub left: bool,
    pub right: bool,
    pub forwards: bool,
    pub back: bool,
    pub shift: bool,
    pub delete: bool,
    pub save: bool,
    pub load: bool
}

impl Controls {
    pub fn right_dragging(&self) -> bool {
        self.right_state.is_dragging()
    }

    pub fn mouse(&self) -> (f32, f32) {
        self.mouse
    }

    pub fn set_mouse(&mut self, x: f32, y: f32) {
        self.mouse = (x, y);
    }

    pub fn update(&mut self) {
        self.save = false;
        self.load = false;

        self.left_state.update(self.mouse);
        self.middle_state.update(self.mouse);
        self.right_state.update(self.mouse);
    }

    pub fn handle_left(&mut self, pressed: bool) {
        self.left_state.handle(self.mouse, pressed);
    }

    pub fn handle_right(&mut self, pressed: bool) {
        self.right_state.handle(self.mouse, pressed);
    }

    pub fn handle_middle(&mut self, pressed: bool) {
        self.middle_state.handle(self.mouse, pressed);
    }

    pub fn middle_clicked(&self) -> bool {
        self.middle_state.is_clicked()
    }

    pub fn left_clicked(&self) -> bool {
        self.left_state.is_clicked()
    }

    pub fn right_clicked(&self) -> bool {
        self.right_state.is_clicked()
    }

    pub fn left_dragged(&self) -> Option<(f32, f32)>  {
        if let MouseState::Dragged(x, y) = self.left_state {
            Some((x, y))
        } else {
            None
        }
    }

    pub fn left_dragging(&self) -> Option<(f32, f32)> {
        if let MouseState::Dragging(x, y) = self.left_state {
            Some((x, y))
        } else {
            None
        }
    }

    pub fn left_drag_rect(&self) -> Option<(f32, f32, f32, f32)> {
        self.left_dragged().map(|(mut left, mut top)| {
            let (mut right, mut bottom) = self.mouse();
            
            if right < left {
                swap(&mut right, &mut left);
            }

            if bottom < top {
                swap(&mut top, &mut bottom);
            }

            (left, top, right, bottom)
        })
    }
}
