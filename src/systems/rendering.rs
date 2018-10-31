use util::*;
use specs::*;
use components;
use context::*;
use circle_size;
use state::*;
use super::*;
use cgmath::Matrix4;

pub struct ObjectRenderer<'a>(pub &'a mut Context);

impl<'a> System<'a> for ObjectRenderer<'a> {
    type SystemData = (
        Read<'a, Camera>,
        Read<'a, StarSystem>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, components::Rotation>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>
    );

    fn run(&mut self, (camera, system, pos, rot, size, model): Self::SystemData) {
        for (pos, rot, size, model) in (&pos, &rot, &size, &model).join() {
            self.0.render_model(*model, pos.0, rot.0, size.0, &camera, &system);
        }
    }
}

pub struct RenderSelected<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderSelected<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Camera>,
        Read<'a, ScreenDimensions>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Side>
    );

    fn run(&mut self, (entities, camera, screen_dims, pos, selectable, size, side): Self::SystemData) {
        for (entity, pos, selectable, side) in (&entities, &pos, &selectable, &side).join() {
            if selectable.selected {
                if let Some((x, y, z)) = camera.screen_position(pos.0, screen_dims.0) {
                    let size = size.get(entity).map(|size| size.0).unwrap_or(1.0);
                    self.0.render_circle(x, y, circle_size(z) * size, side.colour());
                }
            }
        }
    }
}

pub struct RenderCommandPaths<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderCommandPaths<'a> {
    type SystemData = (
        ReadStorage<'a, Position>,
        ReadStorage<'a, Commands>
    );

    fn run(&mut self, (positions, commands): Self::SystemData) {
        for (pos, commands) in (&positions, &commands).join() {
            let points = iter_owned([pos.0])
                .chain(commands.iter().map(|command| command.point(&positions)));

            self.0.render_3d_lines(points, WHITE);
        }
    }
}

pub struct RenderUI<'a>(pub &'a mut Context);

impl<'a> RenderUI<'a> {
    fn render_text(&mut self, text: &str, y: &mut f32) {
        self.0.render_text(text, 10.0, *y);
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
        ReadStorage<'a, Materials>,
        ReadStorage<'a, MineableMaterials>
    );

    fn run(&mut self, (entities, time, formation, paused, tag, selectable, occupation, parent, materials, mineable): Self::SystemData) {
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

        let entity = (&entities, &selectable).join()
            .filter(|(_, selectable)| selectable.selected)
            .map(|(entity, _)| entity)
            .next();

        if let Some(entity) = entity {
            self.render_text("---------------------", y);

            if let Some(materials) = materials.get(entity) {
                self.render_text(&format!("Materials: {}", materials.0), y);
            }

            if let Some(mineable) = mineable.get(entity) {
                self.render_text(&format!("Mineable Materials: {}", mineable.0), y);
            }

            let people = (&occupation, &parent).join()
                .filter(|(_, parent)| parent.0 == entity)
                .map(|(occupation, _)| occupation);

            let (people, total) = summarize(people);

            self.render_text(&format!("Population: {}", total), y);
            
            for (tag, num) in people {
                self.render_text(&format!("{:?}: {}", tag, num), y);
            }
        }
    }
}

pub struct RenderMouse<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderMouse<'a> {
    type SystemData = (
        Read<'a, RightClickOrder>,
        Read<'a, Controls>
    );

    fn run(&mut self, (order, controls): Self::SystemData) {
        let (x, y) = controls.mouse();

        if let Some(Command::GoToAnd(_, interaction)) = order.command {
            self.0.render_image(interaction.image(), x + 32.0, y + 32.0, 64.0, 64.0, [0.0; 4]);
        }
    }
}

pub struct RenderDebug<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderDebug<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Camera>,
        Read<'a, EntityUnderMouse>,
        Read<'a, StarSystem>,
        Read<'a, MouseRay>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, SeekForce>,
        ReadStorage<'a, AvoidanceForce>,
        ReadStorage<'a, FrictionForce>
    );

    fn run(&mut self, (entities, camera, entity, system, ray, pos, vel, seek, avoid, friction): Self::SystemData) {
        if !self.0.is_debugging() {
            return;
        }

        if let Some((_, point)) = entity.0 {
            self.0.render_model(Model::Asteroid, point, Quaternion::zero(), 1.0, &camera, &system);
        }

        if let Some(point) = Plane::new(UP, 0.0).intersection(&ray.0).map(point_to_vector) {
            self.0.render_model(Model::Asteroid, point, Quaternion::zero(), 1.0, &camera, &system);
        }

        let scale = 1000.0;

        for (entity, pos, vel) in (&entities, &pos, &vel).join() {
            let step = Vector3::new(0.0, 0.05, 0.0);
            let mut pos = pos.0 + step;

            if let Some(seek) = seek.get(entity) {
                self.0.render_3d_lines(iter_owned([pos, pos + seek.0 * scale]), [1.0, 0.0, 0.0]);
                pos += step;
            }

            if let Some(avoid) = avoid.get(entity) {
                self.0.render_3d_lines(iter_owned([pos, pos + avoid.0 * scale]), [0.0, 1.0, 0.0]);
                pos += step;
            }

            if let Some(friction) = friction.get(entity) {
                self.0.render_3d_lines(iter_owned([pos, pos + friction.0 * scale]), [0.0, 0.0, 1.0]);
                pos += step;
            }

            self.0.render_3d_lines(iter_owned([pos, pos + vel.0 * scale / 10.0]), [0.0, 1.0, 1.0]);
        }
    }
}

pub struct RenderSystem<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderSystem<'a> {
    type SystemData = (
        Read<'a, Camera>,
        Read<'a, StarSystem>
    );

    fn run(&mut self, (camera, system): Self::SystemData) {
        self.0.render_skybox(&system, &camera);
        self.0.render_stars(&system, &camera);

        let offset = system.light * BACKGROUND_DISTANCE;

        let rotation: Matrix4<f32> = look_at(offset).into();
        let matrix = Matrix4::from_translation(camera.position() + offset) * rotation * Matrix4::from_scale(BACKGROUND_DISTANCE / 10.0);

        self.0.render_billboard(matrix, Image::Star, &camera, &system);
    }
}

pub struct RenderDragSelection<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderDragSelection<'a> {
    type SystemData = Read<'a, Controls>;

    fn run(&mut self, controls: Self::SystemData) {
        if let Some(origin) = controls.left_dragging() {
            self.0.render_rect(origin, controls.mouse());
        }
    }
}

pub struct RenderMovementPlane<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderMovementPlane<'a> {
    type SystemData = Read<'a, RightClickOrder>;

    fn run(&mut self, order: Self::SystemData) {
        if let Some(Command::MoveTo(point)) = order.command {
            let distance = 20.0;

            let point = Vector3::new(round_to(point.x, distance), point.y, round_to(point.z, distance));

            let points = 5;

            let radius = points as f32 * distance / 2.0;

            for i in 0 .. points + 1 {
                let i = i as f32 * distance - radius;

                self.0.render_3d_lines(iter_owned([
                    point + Vector3::new(i, 0.0, -radius),
                    point + Vector3::new(i, 0.0, radius)
                ]), WHITE);

                self.0.render_3d_lines(iter_owned([
                    point + Vector3::new(-radius, 0.0, i),
                    point + Vector3::new(radius, 0.0, i)
                ]), WHITE);
            }
        }
    }
}

pub struct RenderLogSystem<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderLogSystem<'a>  {
    type SystemData = Read<'a, Log>;

    fn run(&mut self, log: Self::SystemData) {
        log.render(&mut self.0);
    }
}

pub struct RenderBillboards<'a>(pub &'a mut Context);

impl<'a> System<'a> for RenderBillboards<'a> {
    type SystemData = (
        Read<'a, Camera>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Image>
    );

    fn run(&mut self, (camera, pos, size, image): Self::SystemData) {
        let rotation = look_at(-camera.direction());

        let len = (&pos, &size, &image).join().count() * 6;

        let iterator = (&pos, &size, &image).join()
            .flat_map(|(pos, size, image)| {
                iter_owned(BILLBOARD_VERTICES).map(move |v| (v, pos.0, size.0, image))
            })
            .map(|(mut v, pos, size, image)| {
                let mut p: Vector3<f32> = v.position.into();
                p *= size;
                p = rotation * p;
                p += pos;
                v.position = p.into();
                v.texture = image.translate(v.texture);
                v
            });

        self.0.render_smoke(iterator, len);
    }
}

pub struct FlushUI<'a>(pub &'a mut Context);

impl<'a> System<'a> for FlushUI<'a> {
    type SystemData = (
        Read<'a, Camera>,
        Read<'a, StarSystem>
    );

    fn run(&mut self, (camera, system): Self::SystemData) {
        self.0.flush_ui(&camera, &system);
    }
}

pub struct FlushSmoke<'a>(pub &'a mut Context);

impl<'a> System<'a> for FlushSmoke<'a> {
    type SystemData = (
        Read<'a, Camera>,
        Read<'a, StarSystem>
    );

    fn run(&mut self, (camera, system): Self::SystemData) {
        self.0.flush_smoke(&system, &camera);
    }
}