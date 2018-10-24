use context::*;
use pedot::*;
use odds::vec::*;

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