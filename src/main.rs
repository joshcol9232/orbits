mod tools;
mod planet;
mod trails;

use ggez::event;
use ggez::graphics::{self, DrawParam, Mesh};
use ggez::nalgebra::{Point2, Vector2};
use ggez::{Context, GameResult};
use ggez::timer;
use ggez::input::mouse::MouseButton;

use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::time::Duration;

use planet::Planet;
use trails::{Emitter, ParticleTrail};

pub const G: f32 = 0.0001;    // Gravitational constant

struct MainState {
    planet_id_count: usize,
    planets: HashMap<usize, RefCell<Planet>>,
    emitters: Vec<Box<dyn Emitter>>,
    planet_trails: HashMap<usize, ParticleTrail>,
    mouse_info: MouseInfo,
}

impl MainState {
    fn new(_ctx: &mut Context) -> GameResult<MainState> {
        let mut s = MainState {
            planet_id_count: 0,
            planets: HashMap::new(),
            emitters: Vec::new(),
            planet_trails: HashMap::new(),
            mouse_info: MouseInfo::default(),
        };

        s.add_planet(
            Point2::new(300.0, 400.0),
            None,
            None,
            30.0
        );

        s.spawn_square_of_planets(
            Point2::new(260.0, 360.0),
            10,
            10,
            50.0,
            2.0,
        );

        s.add_planet(
            Point2::new(600.0, 400.0),
            None,
            None,
            30.0
        );

        Ok(s)
    }

    #[inline]
    fn add_planet(&mut self, position: Point2<f32>, velocity: Option<Vector2<f32>>, mass: Option<f32>, radius: f32) {
        self.add_planet_raw(Planet::new(
            self.planet_id_count,
            position,
            velocity,
            mass,
            radius
        ));
    }

    #[inline]
    fn add_planet_raw(&mut self, mut planet: Planet) {
        planet.id = self.planet_id_count;

        self.planets.insert(
            self.planet_id_count,
            RefCell::new(planet)
        );

        self.planet_trails.insert(
            self.planet_id_count,
            ParticleTrail::new()
        );

        self.planet_id_count += 1;
    }

    #[inline]
    fn remove_planet(&mut self, id: usize) {
        self.planets.remove(&id).expect("Tried to remove planet but it wasn't in the hashmap.");
        if let Some(trail) = self.planet_trails.get_mut(&id) {
            trail.stop_emitting();
        }
    }

    #[inline]
    fn draw_debug_info(&self, ctx: &mut Context) -> GameResult {
        let text = graphics::Text::new(
            format!(
                "{:.3}\nBodies: {}\nPlanet Trails: {}\nParticle Count: {}",
                timer::fps(ctx),
                self.planets.len(),
                self.planet_trails.len(),
                self.particle_count(),
            )
        );
        graphics::draw(
            ctx,
            &text,
            DrawParam::new().dest([10.0, 10.0])
        )
    }

    pub fn draw_mouse_drag(ctx: &mut Context, mouse_info: &MouseInfo) -> GameResult {
        let line = Mesh::new_line(
            ctx,
            &[mouse_info.down_pos, mouse_info.current_drag_position],
            2.0,
            [0.0, 1.0, 0.0, 1.0].into(),
        )?;
        graphics::draw(ctx, &line, DrawParam::default())?;
        tools::draw_circle(ctx, mouse_info.down_pos, 2.0, [1.0, 1.0, 1.0, 0.4].into())?;

        Ok(())
    }

        #[inline]
    fn collide_planets(&self, planets: &HashSet<usize>) -> Planet {  // Returns new planet that is sum of other two.
        // Conservation of momentum
        let mut total_mass = 0.0;
        let mut total_volume = 0.0;
        let mut inital_momentum = Vector2::new(0.0, 0.0);
        let mut sum_of_rm = Point2::new(0.0, 0.0);      // Centre of mass of system is this divided by total mass of all bodies

        for id in planets.iter() {
            let p = self.planets.get(id).expect(&format!("Planet {} not in hashmap.", id)).borrow();
            total_mass += p.mass;
            total_volume += tools::volume_of_sphere(p.radius);
            inital_momentum += p.mass * p.velocity;

            sum_of_rm.x += p.position.x * p.mass;
            sum_of_rm.y += p.position.y * p.mass;
        }

        let new_radius = tools::inverse_volume_of_sphere(total_volume);
        // Use centre of mass as new position
        let new_position = sum_of_rm/total_mass;

        // ID is set to 0, and is then changed afterwards.
        Planet::new(0, new_position, Some(inital_momentum/total_mass), Some(total_mass), new_radius)
    }

    fn spawn_square_of_planets(
        &mut self,
        top_left: Point2<f32>,
        w: u16,
        h: u16,
        gap: f32,
        rad: f32,
    ) {
        for i in 0..w {
            for j in 0..h {
                self.add_planet(
                    Point2::new(top_left.x + i as f32 * gap, top_left.y + j as f32 * gap),
                    None,
                    None,
                    rad,
                );
            }
        }
    }

    fn update_planet_trails(&mut self, dt: f32, dt_duration: &Duration) {
        for (id, trail) in self.planet_trails.iter_mut() {
            trail.update(
                dt,
                dt_duration,
                if let Some(planet) = self.planets.get(&id) {
                    Some(planet.borrow().position)
                } else {
                    None
                }
            );
        }
    }

    fn particle_count(&self) -> usize {
        let mut total = 0;
        for (_, trail) in self.planet_trails.iter() {
            total += trail.particle_count();
        }
        total
    }

    #[inline]
    fn put_in_collision_group(collision_groups: &mut Vec<HashSet<usize>>, i_id: usize, j_id: usize) {
        let mut now_in_group = false;
        for collision_group in collision_groups.iter_mut() {
            let contains_i = collision_group.contains(&i_id);
            let contains_j = collision_group.contains(&j_id);

            if contains_i && contains_j {
                // Do nothing
            } else if contains_i {
                collision_group.insert(j_id);
            } else if contains_j {
                collision_group.insert(i_id);
            }

            if contains_i || contains_j {
                now_in_group = true;
                break
            }
        }

        if !now_in_group {  // Start a new group
            let mut new_set = HashSet::with_capacity(2);
            new_set.insert(i_id);
            new_set.insert(j_id);
            collision_groups.push(new_set);
        }
    }

    #[inline]
    fn resolve_collisions(&mut self, collision_groups: &Vec<HashSet<usize>>) {
        let mut new_planets = Vec::new();
        for collision_group in collision_groups.iter() {
            new_planets.push(self.collide_planets(&collision_group));
            // Remove planets in each collision group (since they will be replaced by new planet)
            for id in collision_group {
                self.remove_planet(*id);
            }
        }

        // Add new planets
        for planet in new_planets {
            self.add_planet_raw(planet);
        }
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let dt_duration = timer::average_delta(ctx);
        let dt = timer::duration_to_f64(dt_duration) as f32;

        /*
            Groups that are colliding.
            E.g: vec![ vec![1, 4, 2], vec![5, 3] ]
        */
        let mut collision_groups: Vec<HashSet<usize>> = Vec::with_capacity(self.planets.len()/2);

        // Remove dead particle trails
        self.planet_trails.retain(|_, trail| !trail.is_dead());

        let keys: Vec<&usize> = self.planets.keys().collect();
        let len = self.planets.len();

        if len > 0 {
            for i in 0..len-1 {
                let pl1 = self.planets.get(keys[i]).expect("Couldn't get planet 1");
                for j in i+1..len {
                    let pl2 = self.planets.get(keys[j]).expect("Couldn't get planet 2");
                    let colliding = {
                        let bpl1 = pl1.borrow();
                        let bpl2 = pl2.borrow();
                        tools::check_collision(&bpl1, &bpl2)
                    };
    
                    if colliding {
                        Self::put_in_collision_group(&mut collision_groups, *keys[i], *keys[j]);
                    } else {
                        tools::newtonian_grav(&mut pl1.borrow_mut(), &mut pl2.borrow_mut());
                    }
                }
            }

            self.resolve_collisions(&collision_groups);
    
            // Update planets
            for (_, pl) in self.planets.iter() {
                pl.borrow_mut().update(dt);
            }
        }

        // Update trails
        self.update_planet_trails(dt, &dt_duration);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 1.0].into());

        if self.mouse_info.down && self.mouse_info.button_down == MouseButton::Left &&
            (self.mouse_info.down_pos.x - self.mouse_info.current_drag_position.x).powi(2) +
            (self.mouse_info.down_pos.y - self.mouse_info.current_drag_position.y).powi(2) >= 4.0
        {
            Self::draw_mouse_drag(ctx, &self.mouse_info)?;
            //self.draw_fake_planet(ctx, self.mouse_info.down_pos, 5.0)?;
        }

        for (_, trail) in self.planet_trails.iter() {
            trail.draw(ctx)?;
        }

        for (_, planet) in self.planets.iter() {
            planet.borrow().draw(ctx)?;
        }

        self.draw_debug_info(ctx)?;
        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        self.mouse_info.down = true;
        self.mouse_info.button_down = button;
        self.mouse_info.down_pos = Point2::new(x, y);
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        self.mouse_info.down = false;

        if button == MouseButton::Left {
            self.add_planet(self.mouse_info.down_pos, Some(self.mouse_info.down_pos - Point2::new(x, y)), None, 2.0);
        }
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.mouse_info.current_drag_position = Point2::new(x, y);
    }
}


struct MouseInfo {
    down: bool,
    button_down: MouseButton,
    down_pos: Point2<f32>,
    current_drag_position: Point2<f32>,
}

impl Default for MouseInfo {
    fn default() -> MouseInfo {
        MouseInfo {
            down: false,
            button_down: MouseButton::Left,
            down_pos: Point2::new(0.0, 0.0),
            current_drag_position: Point2::new(1.0, 0.0),
        }
    }
}

pub fn main() -> GameResult {
    use std::path;
    use std::env;
    use ggez::conf::{WindowMode, WindowSetup, NumSamples};

    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let cb = ggez::ContextBuilder::new("Planets", "ggez")
        .add_resource_path(resource_dir)
        .window_mode(
            WindowMode::default()
                .dimensions(1000.0, 800.0)
        )
        .window_setup(
            WindowSetup::default()
                .samples(NumSamples::Four)
        );

    let (ctx, event_loop) = &mut cb.build()?;
    let state = &mut MainState::new(ctx)?;
    event::run(ctx, event_loop, state)
}