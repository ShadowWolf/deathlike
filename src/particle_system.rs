use crate::{ParticleLifetime, Position, Renderable, Rltk};
use rltk::RGB;
use specs::prelude::*;

pub fn remove_dead_particles(ecs: &mut World, ctx: &Rltk) {
    let dead_particles = find_dead_particles(ecs, ctx);

    for d in dead_particles.iter() {
        ecs.delete_entity(*d)
            .expect("particle could not be deleted");
    }
}

fn find_dead_particles(ecs: &mut World, ctx: &Rltk) -> Vec<Entity> {
    let mut dead_particles: Vec<Entity> = Vec::new();
    {
        let mut particles = ecs.write_storage::<ParticleLifetime>();
        let entities = ecs.entities();
        for (entity, mut particle) in (&entities, &mut particles).join() {
            particle.lifetime_ms -= ctx.frame_time_ms;
            if particle.lifetime_ms < 0. {
                dead_particles.push(entity);
            }
        }
    }
    dead_particles
}

struct ParticleRequest {
    x: i32,
    y: i32,
    fg: RGB,
    bg: RGB,
    glyph: rltk::FontCharType,
    lifetime: f32,
}

pub struct ParticleBuilder {
    requests: Vec<ParticleRequest>,
}

impl ParticleBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> ParticleBuilder {
        ParticleBuilder {
            requests: Vec::new(),
        }
    }

    pub fn request(
        &mut self,
        x: i32,
        y: i32,
        fg: RGB,
        bg: RGB,
        glyph: rltk::FontCharType,
        lifetime: f32,
    ) {
        self.requests.push(ParticleRequest {
            x,
            y,
            fg,
            bg,
            glyph,
            lifetime,
        })
    }
}

pub struct ParticleSpawnSystem {}

impl<'a> System<'a> for ParticleSpawnSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Renderable>,
        WriteStorage<'a, ParticleLifetime>,
        WriteExpect<'a, ParticleBuilder>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut positions,
            mut renderables,
            mut particle_lifetimes,
            mut particle_builders,
        ) = data;

        for new_particle in particle_builders.requests.iter() {
            let p = entities.create();
            positions
                .insert(
                    p,
                    Position {
                        x: new_particle.x,
                        y: new_particle.y,
                    },
                )
                .expect("unable to insert new particle position");
            renderables
                .insert(
                    p,
                    Renderable {
                        bg: new_particle.bg,
                        fg: new_particle.fg,
                        glyph: new_particle.glyph,
                        render_order: 0,
                    },
                )
                .expect("unable to insert new particle renderable");
            particle_lifetimes
                .insert(
                    p,
                    ParticleLifetime {
                        lifetime_ms: new_particle.lifetime,
                    },
                )
                .expect("unable to insert new particle lifetime");
        }

        particle_builders.requests.clear();
    }
}
