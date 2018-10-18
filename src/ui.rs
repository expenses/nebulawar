use context::*;
use pedot::*;
use odds::vec::*;
use state::*;
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

struct LogItem {
    age: f32,
    content: String
}

pub struct UI {
    buttons: [Button; 3],
    log: Vec<LogItem>
}

impl UI {
    pub fn new() -> Self {
        Self {
            buttons: [
                Button::new(HorizontalAlign::Right(0.0), VerticalAlign::Bottom(0.0), Image::Move),
                Button::new(HorizontalAlign::Right(1.0), VerticalAlign::Bottom(0.0), Image::Refuel),
                Button::new(HorizontalAlign::Right(2.0), VerticalAlign::Bottom(0.0), Image::RefuelFrom),
            ],
            log: Vec::new()
        }
    }

    pub fn append_to_log(&mut self, text: String) {
        self.log.push(LogItem {
            age: 0.0,
            content: text
        });
    }

    pub fn step(&mut self, secs: f32) {
        self.log.retain_mut(|item| {
            item.age += secs;
            item.age < 5.0
        });
    }

    pub fn render(&self, state: &State, context: &mut Context) {
        for button in &self.buttons {
            button.render(context);
        }

        let (_, height) = context.screen_dimensions();

        for (i, item) in self.log.iter().enumerate() {
            context.render_text(&item.content, 10.0, height - 30.0 - i as f32 * 20.0);
        }

        let y = &mut 10.0;

        self.render_text(&format!("Time: {:.1}", state.time()), context, y);
        self.render_text(&format!("Ship count: {}", state.ships.len()), context, y);
        self.render_text(&format!("Population: {}", state.people.len()), context, y);

        self.render_text(&format!("Formation: {:?}", state.formation), context, y);        

        for (tag, num) in state.selection_info() {
            self.render_text(&format!("{:?}: {}", tag, num), context, y);
        }

        if let Some(ship) = state.selected().next() {
            self.render_text(&format!("Fuel: {:.2}%", ship.fuel_perc() * 100.0), context, y);
            let (summary, num_people) = summarize(state.people_on_ship(ship.id()).map(|person| person.occupation()));
            
            self.render_text(&format!("Total people: {}", num_people), context, y);

            for (tag, num) in summary {
                self.render_text(&format!("{:?}: {}", tag, num), context, y);
            }

            self.render_text(&format!("Food: {}", ship.food()), context, y);
            self.render_text(&format!("Waste: {}", ship.waste()), context, y);
        }
    }

    fn render_text(&self, text: &str, context: &mut Context, y: &mut f32) {
        context.render_text(text, 10.0, *y);
        *y += 30.0;
    }
}