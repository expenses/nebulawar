use util::*;
use specs::*;
use common_components;
use context::*;
use circle_size;
use state;
use super::*;
use cgmath::Matrix4;

pub struct ObjectRenderer<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for ObjectRenderer<'a> {
    type SystemData = (
        Read<'a, Camera>,
        Read<'a, state::System>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, common_components::Rotation>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>
    );

    fn run(&mut self, (camera, system, pos, rot, size, model): Self::SystemData) {
        for (pos, rot, size, model) in (&pos, &rot, &size, &model).join() {
            self.context.render_model(*model, pos.0, rot.0, size.0, &camera, &system);
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
                .chain(commands.iter().map(|command| command.point(&positions)));

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
        ReadStorage<'a, Parent>,
        ReadStorage<'a, Fuel>
    );

    fn run(&mut self, (entities, time, formation, paused, tag, selectable, occupation, parent, fuel): Self::SystemData) {
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

        let selected = (&entities, &fuel, &selectable).join()
            .filter(|(_, _, selectable)| selectable.selected)
            .map(|(entity, fuel, _)| (entity, fuel))
            .next();

        if let Some((entity, fuel)) = selected {
            self.render_text("---------------------", y);
            self.render_text(&format!("Fuel: {:.2}%", fuel.percentage() * 100.0), y);
            //self.render_text(&format!("Food: {}", storage.food), y);
            //self.render_text(&format!("Waste: {}", storage.waste), y);

            let people = (&occupation, &parent).join()
                .filter(|(_, parent)| parent.0 == entity)
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
        Read<'a, RightClickInteraction>,
        Read<'a, Controls>
    );

    fn run(&mut self, (interaction, controls): Self::SystemData) {
        let (x, y) = controls.mouse();

        if let Some((_, interaction)) = interaction.0 {
            self.context.render_image(interaction.image(), x + 32.0, y + 32.0, 64.0, 64.0, [0.0; 4]);
        }
    }
}

pub struct RenderDebug<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for RenderDebug<'a> {
    type SystemData = (
        Read<'a, Controls>,
        Read<'a, Camera>,
        Read<'a, EntityUnderMouse>,
        Read<'a, state::System>
    );

    fn run(&mut self, (controls, camera, entity, system): Self::SystemData) {
        if !self.context.is_debugging() {
            return;
        }

        if let Some((_, point)) = entity.0 {
            self.context.render_model(Model::Asteroid, point, Quaternion::zero(), 1.0, &camera, &system);
        }

        let ray = self.context.ray(&camera, controls.mouse());
        if let Some(point) = Plane::new(UP, 0.0).intersection(&ray).map(point_to_vector) {
            self.context.render_model(Model::Asteroid, point, Quaternion::zero(), 1.0, &camera, &system);
        }
    }
}

pub struct RenderSystem<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for RenderSystem<'a> {
    type SystemData = (
        Read<'a, Camera>,
        Read<'a, state::System>
    );

    fn run(&mut self, (camera, system): Self::SystemData) {
        self.context.render_skybox(&system, &camera);
        self.context.render_stars(&system, &camera);

        let offset = system.light * BACKGROUND_DISTANCE;

        let rotation: Matrix4<f32> = look_at(offset).into();
        let matrix = Matrix4::from_translation(camera.position() + offset) * rotation * Matrix4::from_scale(BACKGROUND_DISTANCE / 10.0);

        self.context.render_billboard(matrix, Image::Star, &camera, &system);
    }
}

pub struct RenderDragSelection<'a> {
    pub context: &'a mut Context
}

impl<'a> System<'a> for RenderDragSelection<'a> {
    type SystemData = Read<'a, Controls>;

    fn run(&mut self, controls: Self::SystemData) {
        if let Some(origin) = controls.left_dragging() {
            self.context.render_rect(origin, controls.mouse());
        }
    }
}