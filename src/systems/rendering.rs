use util::*;
use specs::*;
use common_components;
use context::*;
use circle_size;
use state;
use super::*;

pub struct ObjectRenderer<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for ObjectRenderer<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Camera>,
        Read<'a, state::System>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, common_components::Rotation>,
        ReadStorage<'a, ObjectSpin>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>
    );

    fn run(&mut self, (entities, camera, system, pos, rot, spin, size, model): Self::SystemData) {
        for (entity, pos, size, model) in (&entities, &pos, &size, &model).join() {
            let rotation = rot.get(entity).map(|rot| rot.0)
                .or(spin.get(entity).map(|spin| spin.to_quat()));

            if let Some(rotation) = rotation {
                self.context.render_model(*model, pos.0, rotation, size.0, &camera, &system);
            }
        }
    }
}

pub struct RenderSelected<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for RenderSelected<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Camera>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Size>
    );

    fn run(&mut self, (entities, camera, pos, selectable, size): Self::SystemData) {
        for (entity, pos, selectable) in (&entities, &pos, &selectable).join() {
            if selectable.selected {
                if let Some((x, y, z)) = self.context.screen_position(pos.0, &camera) {
                    let size = size.get(entity).map(|size| size.0).unwrap_or(1.0);
                    self.context.render_circle(x, y, circle_size(z) * size, [1.0; 4]);
                }
            }
        }
    }
}

pub struct RenderCommandPaths<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for RenderCommandPaths<'a> {
    type SystemData = (
        ReadStorage<'a, Position>,
        ReadStorage<'a, Commands>
    );

    fn run(&mut self, (positions, commands): Self::SystemData) {
        for (pos, commands) in (&positions, &commands).join() {
            let points = iter_owned([pos.0])
                .chain(commands.0.iter().map(|command| command.point(&positions)));

            self.context.render_3d_lines(points);
        }
    }
}

pub struct RenderUI<'a> {
    pub context: &'a mut Context
}

impl<'a> RenderUI<'a> {
    fn render_text(&mut self, text: &str, y: &mut f32) {
        self.context.render_text(text, 10.0, *y);
        *y += 30.0;
    }
}

impl<'a> System<'a> for RenderUI<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Time>,
        Read<'a, Formation>,
        Read<'a, Paused>,
        ReadStorage<'a, ShipType>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Occupation>,
        ReadStorage<'a, Location>,
        ReadStorage<'a, ShipStorage>
    );

    fn run(&mut self, (entities, time, formation, paused, tag, selectable, occupation, location, storage): Self::SystemData) {
        let y = &mut 10.0;

        if paused.0 {
            self.render_text("PAUSED", y);
        }
        self.render_text(&format!("Time: {:.1}", time.0), y);
        self.render_text(&format!("Population: {}", occupation.join().count()), y);
        self.render_text(&format!("Formation: {:?}", *formation), y);

        let (ships, ships_total) = summarize(tag.join());

        self.render_text(&format!("Ship count: {}", ships_total), y);

        for (tag, num) in ships {
            self.render_text(&format!("{:?}: {}", tag, num), y);
        }

        let selected = (&entities, &storage, &selectable).join()
            .filter(|(_, _, selectable)| selectable.selected)
            .map(|(entity, storage, _)| (entity, storage))
            .next();

        if let Some((entity, storage)) = selected {
            self.render_text("---------------------", y);
            self.render_text(&format!("Fuel: {:.2}%", storage.fuel.percentage() * 100.0), y);
            self.render_text(&format!("Food: {}", storage.food), y);
            self.render_text(&format!("Waste: {}", storage.waste), y);

            let people = (&occupation, &location).join()
                .filter(|(_, location)| location.0 == entity)
                .map(|(occupation, _)| occupation);

            let (people, total) = summarize(people);

            self.render_text(&format!("Ship Population: {}", total), y);
            
            for (tag, num) in people {
                self.render_text(&format!("{:?}: {}", tag, num), y);
            }
        }
    }
}

//todo: get runic to batch rendering

pub struct RenderMouse<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for RenderMouse<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Mouse>,
        Read<'a, Camera>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, ShipStorage>,
        ReadStorage<'a, MineableMaterials>
    );

    fn run(&mut self, (entities, mouse, camera, pos, sizes, storage, mineable): Self::SystemData) {
        let (x, y) = mouse.0;

        if let Some(entity) = entity_at(x, y, &entities, &pos, &sizes, &self.context, &camera) {
            if let Some(interaction) = right_click_interaction(entity, &storage, &mineable) {
                self.context.render_image(interaction.image(), x + 32.0, y + 32.0, 64.0, 64.0, [0.0; 4]);
            }
        }
    }
}