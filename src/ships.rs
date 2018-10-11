use std::collections::*;
use std::collections::hash_map::*;
use std::ops::*;

use cgmath::*;
use camera::*;
use *;

pub enum Command {
    MoveTo(Vector3<f32>)
}

pub enum ShipType {
    Fighter,
    Tanker
}

impl ShipType {
    fn model(&self) -> usize {
        match *self {
            ShipType::Fighter => 0,
            ShipType::Tanker => 1
        }
    }
}

pub struct Ship {
    id: usize,
    tag: ShipType,
    pub position: Vector3<f32>,
    angle: Quaternion<f32>,
    pub commands: Vec<Command>
}

impl Ship {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn position_matrix(&self) -> Matrix4<f32> {
        let angle: Matrix4<f32> = self.angle.into();
        Matrix4::from_translation(self.position) * angle
    }

    pub fn step(&mut self) {
        let mut clear = false;
        if let Some(Command::MoveTo(position)) = self.commands.first() {
            let delta = position - self.position;

            if 0.5 < self.position.distance(*position) {
                let step = delta.normalize_to(0.5);

                self.position += step;
            } else {
                self.position = *position;
                clear = true;
            }

            let ideal = Quaternion::look_at(delta, Vector3::new(0.0, 1.0, 0.0)).invert();

            self.angle = ideal;
        }

        if clear {
            self.commands.clear();
        }
    }

    pub fn render(&self, context: &mut context::Context, camera: &Camera, world: &World) {
        context.render(self.tag.model(), self.position_matrix(), camera, world.light);
    }
}

#[derive(Default)]
pub struct Ships {
    inner: HashMap<usize, Ship>,
    next_id: usize
}

impl Ships {
    pub fn push(&mut self, tag: ShipType, position: Vector3<f32>, angle: (f32, f32, f32)) {
        let (pitch, yaw, roll) = angle;

        self.inner.insert(self.next_id, Ship {
            id: self.next_id,
            tag, position,
            angle: Euler::new(Rad(pitch), Rad(yaw), Rad(roll)).into(),
            commands: Vec::new()
        });

        self.next_id += 1;
    }

    /*fn get(&self, id: usize) -> Option<&Ship> {
        self.inner.get(&id)
    }*/

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Ship> {
        self.inner.get_mut(&id)
    }

    pub fn iter(&self) -> Values<usize, Ship> {
        self.inner.values()
    }

    pub fn iter_mut(&mut self) -> ValuesMut<usize, Ship> {
        self.inner.values_mut()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl Index<usize> for Ships {
    type Output = Ship;

    fn index(&self, index: usize) -> &Ship {
        &self.inner[&index]
    }
}

impl IndexMut<usize> for Ships {
    fn index_mut(&mut self, index: usize) -> &mut Ship {
        self.get_mut(index).unwrap()
    }
}