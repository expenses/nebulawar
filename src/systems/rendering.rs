use crate::util::*;
use specs::*;
use crate::components;
use crate::context::*;
use super::*;
use cgmath::Matrix4;

pub struct ObjectRenderer;

impl<'a> System<'a> for ObjectRenderer {
    type SystemData = (
        Write<'a, Buffers>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, components::Rotation>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>
    );

    fn run(&mut self, (mut buffers, pos, rot, size, model): Self::SystemData) {
        for (pos, rot, size, model) in (&pos, &rot, &size, &model).join() {
            let scale = Matrix4::from_scale(size.0);
            let rotation: Matrix4<f32> = rot.0.into();
            let position = Matrix4::from_translation(pos.0) * rotation * scale;
            let instance = InstanceVertex::new(position);
            buffers.push_model(*model, instance);
        }
    }
}

pub struct RenderSystem;

impl<'a> System<'a> for RenderSystem {
    type SystemData = (
        Read<'a, Camera>,
        Read<'a, StarSystem>,
        Write<'a, Buffers>,
    );

    fn run(&mut self, (camera, system, mut buffers): Self::SystemData) {
        let offset = system.light * BACKGROUND_DISTANCE;

        let rotation: Matrix4<f32> = look_at(offset).into();
        let matrix = Matrix4::from_translation(camera.position() + offset) * rotation * Matrix4::from_scale(BACKGROUND_DISTANCE / 10.0);

        buffers.push_billboard(matrix, Image::Star);
    }
}

pub struct RenderSelected;

impl<'a> System<'a> for RenderSelected {
    type SystemData = (
        Entities<'a>,
        Read<'a, Camera>,
        Write<'a, Buffers>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Side>
    );

    fn run(&mut self, (entities, camera, mut buffers, pos, selectable, size, side): Self::SystemData) {
        for (entity, pos, selectable, side) in (&entities, &pos, &selectable, &side).join() {
            if selectable.selected {
                let size = size.get(entity).map(|size| size.0).unwrap_or(1.0);
                buffers.push_circle(pos.0, size, side.colour(), &camera);
            }
        }
    }
}


pub struct RenderCommandPaths;

impl<'a> System<'a> for RenderCommandPaths {
    type SystemData = (
        Write<'a, Buffers>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Commands>
    );

    fn run(&mut self, (mut buffers, positions, selectable, commands): Self::SystemData) {
        (&positions, &selectable, &commands).join()
            .filter(|(_, selectable, _)| selectable.selected)
            .for_each(|(pos, _, commands)| {
                let points = iter_owned([pos.0])
                    .chain(commands.iter().filter_map(|command| command.point(&positions)));

                buffers.push_3d_lines(points, WHITE);
            });
    }
}

pub struct RenderUI;

impl<'a> System<'a> for RenderUI {
    type SystemData = (
        Entities<'a>,
        Write<'a, Buffers>,
        Read<'a, Time>,
        Read<'a, Formation>,
        Read<'a, Paused>,
        Read<'a, Help>,
        ReadStorage<'a, ShipType>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Occupation>,
        ReadStorage<'a, Parent>,
        ReadStorage<'a, Materials>,
        ReadStorage<'a, MineableMaterials>,
        ReadStorage<'a, Health>
    );

    fn run(&mut self, (entities, mut buffers, time, formation, paused, help, tag, selectable, occupation, parent, materials, mineable, health): Self::SystemData) {
        let y = &mut 10.0;

        let mut render_text = |text: &str|  {
            buffers.push_text(text, 10.0, *y);
            *y += 20.0;
        };

        if help.0 {
            render_text("Controls:");
            render_text("WASD to move camera");
            render_text("Drag the right mouse button to rotate the camera");
            render_text("Scroll with the mouse wheel to move the camera closer or further away");
            render_text("Click or drag with the left mouse button to select ships");
            render_text("Hold shift while clicking/dragging to add to the selection");
            render_text("Press C or click the middle mouse button to center the camera on the selected ships");
            render_text("Right click the mouse to order the ships to do something");
            render_text("Holding shift while right clicking will queue orders");
            render_text("Holding shift while moving the mouse up and down will move the plane of movement vertically");
            render_text("Press , and . to rotate through the formation list");
            render_text("Press P to pause/unpause");
            render_text("Press / for the debug view");
            render_text("Press H to toggle this text");
            render_text("---------------------------");
        }

        if paused.0 {
            render_text("PAUSED");
        }

        render_text(&format!("Time: {:.1}", time.0));
        render_text(&format!("Population: {}", occupation.join().count()));
        render_text(&format!("Formation: {:?}", *formation));

        let (ships, ships_total) = summarize(tag.join());

        render_text(&format!("Ship count: {}", ships_total));

        for (tag, num) in ships {
            render_text(&format!("{:?}: {}", tag, num));
        }

        let entity = (&entities, &selectable).join()
            .filter(|(_, selectable)| selectable.selected)
            .map(|(entity, _)| entity)
            .next();

        if let Some(entity) = entity {
            render_text("---------------------");

            if let Some(health) = health.get(entity) {
                render_text(&format!("Health: {}", health.0));
            }

            if let Some(materials) = materials.get(entity) {
                render_text(&format!("Materials: {}", materials.0));
            }

            if let Some(mineable) = mineable.get(entity) {
                render_text(&format!("Mineable Materials: {}", mineable.0));
            }

            let people = (&occupation, &parent).join()
                .filter(|(_, parent)| parent.0 == entity)
                .map(|(occupation, _)| occupation);

            let (people, total) = summarize(people);

            render_text(&format!("Population: {}", total));
            
            for (tag, num) in people {
                render_text(&format!("{:?}: {}", tag, num));
            }
        }
    }
}


pub struct RenderMouse;

impl<'a> System<'a> for RenderMouse {
    type SystemData = (
        Write<'a, LineBuffers>,
        Read<'a, RightClickOrder>,
        Read<'a, Controls>,
        Read<'a, ScreenDimensions>,
    );

    fn run(&mut self, (mut buffers, order, controls, screen_dims): Self::SystemData) {
        let (x, y) = controls.mouse();

        if let Some(Command::GoToAnd(_, interaction)) = order.command {
            buffers.push_image(interaction.image(), x + 32.0, y + 32.0, 64.0, 64.0, [0.0; 4], screen_dims.0.into());
        }
    }
}

pub struct RenderDebug;

impl<'a> System<'a> for RenderDebug {
    type SystemData = (
        Entities<'a>,
        Write<'a, Buffers>,
        Read<'a, Camera>,
        Read<'a, EntityUnderMouse>,
        Read<'a, Debug>,
        Read<'a, Meshes>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, components::Rotation>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, SeekForce>,
        ReadStorage<'a, AvoidanceForce>,
        ReadStorage<'a, FrictionForce>
    );

    fn run(&mut self, (entities, mut buffers, camera, entity, debug, meshes, pos, rot, size, model, vel, seek, avoid, friction): Self::SystemData) {
        if !debug.0 {
            return;
        }

        if let Some((_, point)) = entity.0 {
            buffers.push_circle(point, 10.0, [1.0; 3], &camera);
        }

        let scale = 1000.0;

        for (entity, pos, rot, size, model, vel) in (&entities, &pos, &rot, &size, &model, &vel).join() {
            let step = Vector3::new(0.0, 0.05, 0.0);
            let mut position = pos.0 + step;

            if let Some(seek) = seek.get(entity) {
                buffers.push_3d_line(position, position + seek.0 * scale, [1.0, 0.0, 0.0]);
                position += step;
            }

            if let Some(avoid) = avoid.get(entity) {
                buffers.push_3d_line(position, position + avoid.0 * scale, [0.0, 1.0, 0.0]);
                position += step;
            }

            if let Some(friction) = friction.get(entity) {
                buffers.push_3d_line(position, position + friction.0 * scale, [0.0, 0.0, 1.0]);
                position += step;
            }

            buffers.push_3d_line(position, position + vel.0 * scale / 10.0, [0.0, 1.0, 1.0]);

            // render bbox

            let bbox = meshes.get_bbox(*model, pos.0, rot.0, size.0);

            let min = na_point_to_vector(*bbox.mins());
            let max = na_point_to_vector(*bbox.maxs());

            for i in 0 .. 3 {
                let start = min;
                let mut end = start;
                end[i] = max[i];
                buffers.push_3d_line(start, end, WHITE);

                let start = max;
                let mut end = start;
                end[i] = min[i];

                buffers.push_3d_line(start, end, WHITE);
            }
        }
    }
}


pub struct RenderDragSelection;

impl<'a> System<'a> for RenderDragSelection {
    type SystemData = (Write<'a, LineBuffers>, Read<'a, Controls>, Read<'a, ScreenDimensions>);

    fn run(&mut self, (mut buffers, controls, screen_dims): Self::SystemData) {
        if let Some(origin) = controls.left_dragging() {
            buffers.push_rect(origin, controls.mouse(), screen_dims.0.into());
        }
    }
}


pub struct RenderMovementPlane;

impl<'a> System<'a> for RenderMovementPlane {
    type SystemData = (Write<'a, Buffers>, Read<'a, RightClickOrder>);

    fn run(&mut self, (mut buffers, order): Self::SystemData) {
        if let Some(Command::MoveTo(point)) = order.command {
            let distance = 20.0;

            let point = Vector3::new(round_to(point.x, distance), point.y, round_to(point.z, distance));

            let points = 5;

            let radius = points as f32 * distance / 2.0;

            for i in 0 .. points + 1 {
                let i = i as f32 * distance - radius;

                buffers.push_3d_line(
                    point + Vector3::new(i, 0.0, -radius),
                    point + Vector3::new(i, 0.0, radius),
                    WHITE
                );

                buffers.push_3d_line(
                    point + Vector3::new(-radius, 0.0, i),
                    point + Vector3::new(radius, 0.0, i),
                    WHITE
                );
            }
        }
    }
}

pub struct RenderLogSystem;

impl<'a> System<'a> for RenderLogSystem  {
    type SystemData = (Write<'a, Buffers>, Read<'a, Log>, Read<'a, ScreenDimensions>);

    fn run(&mut self, (mut buffers, log, screen_dims): Self::SystemData) {
        log.render(&mut buffers, (screen_dims.0).1);
    }
}


pub struct RenderBillboards;

impl<'a> System<'a> for RenderBillboards {
    type SystemData = (
        Read<'a, Camera>,
        Write<'a, Buffers>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Image>
    );

    fn run(&mut self, (camera, mut buffers, pos, size, image): Self::SystemData) {
        let cam_pos = camera.position();
        let rotation = look_at(-camera.direction());

        let pred = |point: &Position| point.distance2(cam_pos);

        let mut billboards: Vec<_> = (&pos, &size, &image).join().collect();
        billboards.sort_unstable_by(|a, b| cmp_floats(pred(a.0), pred(b.0)));

        for (pos, size, image) in billboards {
            let scale = Matrix4::from_scale(size.0);
            let rotation: Matrix4<f32> = rotation.into();
            let position = Matrix4::from_translation(pos.0) * rotation * scale;
            buffers.push_billboard(position, *image);
        }
    }
}
