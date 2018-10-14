use context::*;
use pedot::*;
use util::*;

pub struct Button {
    x: HorizontalAlign,
    y: VerticalAlign,
    image: Image
}

impl Button {
    pub fn new(x: HorizontalAlign, y: VerticalAlign, image: Image) -> Self {
        Self {
            x, y, image
        }
    }

    fn location(&self, context: &Context) -> (f32, f32, f32, f32) {
        let (width, height) = context.resources.image_dimensions(self.image);
        let (width, height) = (width as f32 * 2.0, height as f32 * 2.0);

        let x = context.gui.x_absolute(self.x * width + width / 2.0);
        let y = context.gui.y_absolute(self.y * height + height / 2.0);

        (x, y, width, height)
    }

    fn state(&self, context: &Context) -> ButtonState {
        let (x, y, width, height) = self.location(context);
        context.gui.button(x, y, width, height)
    }

    pub fn render(&self, context: &mut Context) {
        let (x, y, width, height) = self.location(context);

        let color = if self.state(context).is_hovering() {
            [0.0, 0.0, 0.0, 0.25]
        } else {
            [0.0; 4]
        };

        context.render_image(Image::Button, x, y, width, height, color);
        context.render_image(self.image, x, y, width, height, color);
    }
}

pub struct UI {
    buttons: [Button; 3]
}

impl UI {
    pub fn new() -> Self {
        Self {
            buttons: [
                Button::new(HorizontalAlign::Right(0.0), VerticalAlign::Bottom(0.0), Image::Move),
                Button::new(HorizontalAlign::Right(1.0), VerticalAlign::Bottom(0.0), Image::Refuel),
                Button::new(HorizontalAlign::Right(2.0), VerticalAlign::Bottom(0.0), Image::RefuelFrom),
            ]
        }
    }

    pub fn button_clicked(&self, context: &Context) -> Option<CommandType> {
        self.buttons.iter()
            .zip(iter_owned([CommandType::Move, CommandType::Refuel, CommandType::RefuelOther]))
            .find(|(button, _)| button.state(context).is_clicked())
            .map(|(_, which)| which)
    }

    pub fn render(&self, context: &mut Context) {
        for button in &self.buttons {
            button.render(context);
        }
    }
}

pub enum CommandType {
    Move,
    Refuel,
    RefuelOther
}