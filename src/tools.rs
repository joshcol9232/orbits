use ggez::graphics::{self, Mesh, Canvas, DrawMode, DrawParam};
use ggez::{Context, GameResult};

use nalgebra::{Vector2, Point2};

use std::f32::consts::PI;
use crate::{G, planet::Planet};

pub fn volume_of_sphere(radius: f32) -> f32 {
  (4.0/3.0) * PI * radius.powi(3)
}

pub fn inverse_volume_of_sphere(volume: f32) -> f32 {
  ((3.0 * volume)/(4.0 * PI)).powf(1.0/3.0)
}

pub fn get_angle(vec: Vector2<f32>) -> f32 {
  vec.y.atan2(vec.x)
}

pub fn get_components(magnitude: f32, angle: f32) -> Vector2<f32> {
  Vector2::new(magnitude * angle.cos(), magnitude * angle.sin())
}

// F = (GMm/|r|^2) * r_norm
//   = (GMm/|r|^2) * r * 1/|r|
//   = (GMm/|r|^3) * r
pub fn newtonian_grav(pl1: &mut Planet, pl2: &mut Planet, dist_squared: f32, dist_vec: Vector2<f32>) {
  let force_vec = dist_vec * (G * pl1.mass * pl2.mass/dist_squared.sqrt().powi(3));

  pl1.resultant_force += force_vec;
  pl2.resultant_force -= force_vec;
}

// Returns the magnitude of the velocity (speed) needed for a circular orbit around another planet
// Orbit is circular when the kinetic energy does not change.
// K = GMm/2r  -- Derived from centripetal force (in circular motion) = gravitational force
// GMm/2r = 1/2 mv^2
// GM/2r = 1/2 v^2
// sqrt(GM/r) = v
pub fn circular_orbit_speed(host_mass: f32, radius: f32) -> f32 {
  (G * host_mass/radius).sqrt()
}