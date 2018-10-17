use cgmath::*;
use util::*;
use std::f32::consts::*;

pub enum Formation {
    Screen,
    DeltaWing
}

impl Formation {
    pub fn arrange(&self, ships: usize, position: Vector3<f32>, target: Vector3<f32>, distance: f32) -> Vec<Vector3<f32>> {
        let mut step = target - position;
        step.y = 0.0;
        let step = step.normalize_to(distance);

        let step_sideways = Quaternion::from_angle_y(Rad(FRAC_PI_2)).rotate_vector(step);

        match *self {
            Formation::Screen => {                
                let step_up = UP * distance;

                let width = (ships as f32).sqrt().ceil() as usize;

                let middle_x = (width - 1) as f32 / 2.0;

                let middle_y = (ships as f32 / width as f32).floor() / 2.0;

                (0 .. ships)
                    .map(|i| {
                        let x = (i % width) as f32 - middle_x;
                        let y = (i / width) as f32 - middle_y;

                        target + step_sideways * x + step_up * y
                    })
                    .collect()
            },
            Formation::DeltaWing => {
                let middle_x = (ships - 1) as f32 / 2.0;

                (0 .. ships)
                    .map(|i| {
                        let x = i as f32 - middle_x;

                        let y = -(i as f32 - middle_x).abs();

                        target + step * y + step_sideways * x
                    })
                    .collect()
            }
        }
    }
}
