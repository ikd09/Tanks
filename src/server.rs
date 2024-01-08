use rand::Rng;
use ambient_api::{
    core::{
        primitives::components::{quad, cube},
        transform::{components::{translation, scale, rotation, euler_rotation}, concepts::Transformable}, rendering::components::{color, pbr_material_from_url, transparency_group}, physics::components::{cube_collider, dynamic, angular_velocity, linear_velocity, physics_controlled, collider_from_url, sphere_collider, visualize_collider}, player::components::is_player, messages::Collision, model::components::model_from_url,
    },
    prelude::*, physics::{add_force_at_position, set_gravity, add_force}, rand, glam::EulerRot,
};
use packages::this::{messages::Movement, messages::{Bonk, Respawn}, components::{turret_ref, bullet_movement, player_targetter_ref, game_player, last_shot_time, bullet_fire_timer, smoke_dissipation_delay, time_between_treadmarks, treadmark_display_timer, time_of_last_smoke_emission, smoke_trail_velocity, kills_to_deaths, track_spawn_time, tank_death_time, bullet_owner, tank_vertical_velocity}, assets};

use crate::packages::this::{messages::{SummonExplosion}, components::{bullet_bounce_count, last_hit_normal}};

#[main]
pub fn main() {

    Entity::new()
        .with(model_from_url(), assets::url("Battleground.glb"))
        .with(scale(), vec3(35.0, 35.0, 1.0))
        // .with(color(), Vec4::ONE)
        .with(collider_from_url(), assets::url("Battleground.glb"))
        .with(translation(), vec3(0.0, 0.0, -2.0))
        .spawn();
    Entity::new()
        .with(scale(), vec3(500.0, 500.0, 1.0))
        .with(cube_collider(), Vec3::ONE)
        .with(translation(), vec3(0.0, 0.0, -3.0))
        .spawn();

    set_gravity(vec3(0.0, 0.0, -20.0));

    spawn_query(is_player()).bind(move |players| {

        let mut rng = rand::thread_rng();

        for (id, _) in players {

            let turret = Entity::new()
                .with(model_from_url(), assets::url("TankTurret.glb"))
                .with(translation(), Vec3::ZERO)
                .with(euler_rotation(), Vec3::ZERO)
                .with(cube_collider(), vec3(1.0, 0.8, 0.8))
                .with(linear_velocity(), Vec3::ZERO)
                .with(angular_velocity(), Vec3::ZERO)
                .with(dynamic(), true)
                .with(scale(), Vec3::ONE * 0.5)
                .spawn();

            entity::add_components(
                id,
                Entity::new()
                    .with(game_player(), id)
                    .with(model_from_url(), assets::url("TankHull.glb"))
                    .with(collider_from_url(), assets::url("TankHull.glb"))
                    .with(translation(), vec3(rng.gen_range(-0.3..0.3) * 100.0, 0.0, -1.0))
                    .with(euler_rotation(), vec3(0.0, 0.0, 0.0))
                    .with(linear_velocity(), Vec3::ZERO)
                    .with(angular_velocity(), Vec3::ZERO)
                    .with(kills_to_deaths(), vec2(0.0, 0.0))
                    .with(tank_vertical_velocity(), 0.0)
                    .with(physics_controlled(), ())
                    .with(dynamic(), true)
                    .with(last_shot_time(), game_time())
                    .with(tank_death_time(), game_time())
                    .with(scale(), vec3(1.0, 1.2, 0.5))
                    .with(time_between_treadmarks(), game_time())
            );

            entity::add_component(
                id,
                turret_ref(),
                turret
            );

            entity::add_component(
                id,
                player_targetter_ref(),
                spawn_cube(Vec3::ZERO),
            );
        }
    });

    Respawn::subscribe(move | ctx, mut msg | {
        let client = ctx.client_entity_id().unwrap();

        if msg.sent_respawn_command == true {
            entity::add_component(client, game_player(), client);
            msg.sent_respawn_command = false;
            entity::set_component(client, scale(), vec3(1.0, 1.2, 0.7));
            let turret = entity::get_component(client, turret_ref()).unwrap();
            entity::set_component(turret, scale(), Vec3::ONE * 0.5);
        }
    });

    Collision::subscribe(move |msg| {
        for vector in msg.normals {
            for (index, object_id) in msg.ids.iter().enumerate() {
                let next_index = (index + 1) % msg.ids.len();
                let next_object_id = msg.ids[next_index];

                if entity::has_component(*object_id, time_of_last_smoke_emission()) && entity::has_component(next_object_id, time_of_last_smoke_emission()) {
                    SummonExplosion {
                        translation: entity::get_component(*object_id, translation()).unwrap(),
                        scale: 1.0
                    }.send_client_broadcast_unreliable();
                    entity::despawn(*object_id);
                    entity::despawn(next_object_id);
                } 

                if entity::has_component(*object_id, time_of_last_smoke_emission()) {

                    if entity::get_component(*object_id, last_hit_normal()).unwrap() == vector {
                        return;
                    }

                    let bullet_dv = entity::get_component(*object_id, bullet_movement()).unwrap();

                    let reflected_direction = reflect(&bullet_dv, &vector);

                    entity::set_component(*object_id, bullet_movement(), reflected_direction);
                    entity::set_component(*object_id, angular_velocity(), Vec3::ZERO);
                    let bounce_count = entity::get_component(*object_id, bullet_bounce_count()).unwrap();
                    entity::set_component(*object_id, bullet_bounce_count(), bounce_count + 1);
                    if entity::has_component(next_object_id, game_player()) && entity::has_component(next_object_id, tank_death_time()) {
                        entity::remove_component(next_object_id, game_player());
                        entity::set_component(next_object_id, tank_death_time(), game_time());
                        let current_kd = entity::get_component(next_object_id, kills_to_deaths()).unwrap();
                        entity::set_component(next_object_id, kills_to_deaths(), vec2(current_kd.x, current_kd.y + 1.0));
                        let projectile_owner = entity::get_component(*object_id, bullet_owner()).unwrap();
                        if !(next_object_id == projectile_owner) {
                            println!("You were shot by a foreign bullet!");
                            let current_enemy_kd = entity::get_component(projectile_owner, kills_to_deaths()).unwrap();
                            entity::set_component(projectile_owner, kills_to_deaths(), vec2(current_enemy_kd.x + 1.0, current_enemy_kd.y));
                        }
                        SummonExplosion {
                            translation: entity::get_component(*object_id, translation()).unwrap(),
                            scale: 3.0,
                        }.send_client_broadcast_unreliable();
                        Bonk {
                            collision_sound_effect: 0,
                            emitter: next_object_id,
                        }.send_client_broadcast_unreliable();
                        entity::despawn(*object_id);
                        entity::set_component(next_object_id, scale(), Vec3::ZERO);
                        let turret = entity::get_component(next_object_id, turret_ref()).unwrap();
                        entity::set_component(turret, scale(), Vec3::ZERO);
                        return;
                    }
                    Bonk {
                        collision_sound_effect: 3,
                        emitter: next_object_id,
                    }.send_client_broadcast_unreliable();
                    entity::set_component(*object_id, last_hit_normal(), vector);
                    SummonExplosion {
                        translation: entity::get_component(*object_id, translation()).unwrap(),
                        scale: 1.0
                    }.send_client_broadcast_unreliable();
                    if bounce_count + 1 >= 3 {
                        entity::despawn(*object_id);
                    }
                    return;
                    
                }
                // if entity::has_component(next_object_id, time_of_last_smoke_emission()) {

                //     let bullet_dv = entity::get_component(next_object_id, bullet_movement()).unwrap();

                //     let reflected_direction = reflect(&bullet_dv, &vector);

                //     entity::set_component(next_object_id, bullet_movement(), reflected_direction);
                //     let bounce_count = entity::get_component(next_object_id, bullet_bounce_count()).unwrap();
                //     entity::set_component(next_object_id, bullet_bounce_count(), bounce_count + 1);
                //     Bonk {
                //         collision_sound_effect: 3,
                //         emitter: next_object_id,
                //     }.send_client_broadcast_unreliable();
                //     SummonExplosion {
                //         translation: entity::get_component(next_object_id, translation()).unwrap(),
                //         scale: 1.0
                //     }.send_client_broadcast_unreliable();
                //     if bounce_count + 1 >= 3 {
                //         entity::despawn(next_object_id);
                //     }
                //     return;
                // }
            }
        }
    });


    Movement::subscribe(|ctx, mut msg| {

        let mut rng = rand::thread_rng();

        if ctx.client_user_id().is_none() || ctx.client_entity_id().is_none() {
            return;
        }

        let client = ctx.client_entity_id().unwrap();

        if !entity::has_component(client, game_player()) {
            return;
        }

        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        let Some(cube_id) = entity::get_component(player_id, player_targetter_ref()) else {
            return;
        };

        let Some(hit) = physics::raycast_first(msg.ray_origin, msg.ray_dir) else {
            return;
        };
        // Set position of cube to the raycast hit position
        entity::set_component(cube_id, translation(), hit.position);

        let turret = entity::get_component(client, turret_ref()).unwrap();

        let turret_position = entity::get_component(turret, translation()).unwrap();

        let delta_y: f32 = turret_position.y - hit.position.y;
        let delta_x = turret_position.x - hit.position.x;

        let angle = delta_y.atan2(delta_x);

        let e_translation = entity::get_component(client, translation()).unwrap();

        entity::set_component(turret, translation(), vec3(e_translation.x, e_translation.y, e_translation.z + 1.0));
        let mut setting_angle = Vec3::Z * angle;
        setting_angle = vec3(setting_angle.x, setting_angle.y, setting_angle.z - 135.0);

        entity::set_component(turret, euler_rotation(), setting_angle);

        let e_rotation_quat = entity::get_component(client, rotation()).unwrap();

        let e_rotation_nonvec = e_rotation_quat.to_euler(EulerRot::XYZ);

        let e_rotation = vec3(e_rotation_nonvec.0, e_rotation_nonvec.1, e_rotation_nonvec.2);

        let movement_vector = vec2((e_rotation.z.to_degrees() + 90.0).to_radians().cos(), (e_rotation.z.to_degrees() + 90.0).to_radians().sin());

        if msg.fire == true {

            if !(game_time().as_secs_f32() - entity::get_component(client, last_shot_time()).unwrap().as_secs_f32() >= 1.0) {
                return;
            }
            entity::set_component(client, last_shot_time(), game_time());
            msg.fire = false;
            let direction_vector = vec3(angle.cos(), angle.sin(), 0.0) * -1.0;
            let bullet_spawn_location = direction_vector * 3.0 + e_translation;

            Entity::new()
                .with_merge(Transformable::suggested())
                .with(physics_controlled(), ())
                .with(sphere_collider(), 1.3)
                .with(model_from_url(), assets::url("Pellet.glb"))
                .with(translation(), bullet_spawn_location)
                .with(rotation(), Quat::from_rotation_z(angle + std::f32::consts::PI))
                .with(scale(), Vec3::ONE * 0.25)
                .with(linear_velocity(), Vec3::ZERO)
                .with(bullet_owner(), client)
                .with(last_hit_normal(), Vec3::ZERO)
                .with(angular_velocity(), Vec3::ZERO)
                .with(bullet_movement(), direction_vector)
                .with(bullet_bounce_count(), 0)
                .with(dynamic(), true)
                .with(time_of_last_smoke_emission(), game_time())
                .spawn();            
            let mut explosion_spawn_location = e_translation + direction_vector * 2.0;
            
            explosion_spawn_location.z = 2.0;
            Bonk {
                collision_sound_effect: 1,
                emitter: client,
            }.send_client_broadcast_unreliable();
            for _i in 1..3 {
                Entity::new()
                    .with(cube(), ())
                    .with(translation(), explosion_spawn_location + rng.gen_range(-1.0..1.0))
                    .with(smoke_dissipation_delay(), rng.gen_range(0.1..0.3) * 0.1)
                    .with(scale(), vec3(rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0)))
                    .with(rotation(), Quat::from_euler(EulerRot::XYZ, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0))
                    .with(smoke_trail_velocity(), direction_vector * 0.2)
                    .with(bullet_fire_timer(), game_time())
                    .with(color(), vec4(rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), 0.7))
                    .with(transparency_group(), 0)
                    .spawn();
            }
            for _i in 1..3 {
                Entity::new()
                    .with(cube(), ())
                    .with(translation(), explosion_spawn_location + rng.gen_range(-1.0..1.0))
                    .with(smoke_dissipation_delay(), rng.gen_range(0.1..0.3))
                    .with(scale(), vec3(rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0)))
                    .with(rotation(), Quat::from_euler(EulerRot::XYZ, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0))
                    .with(smoke_trail_velocity(), direction_vector * 0.2)
                    .with(bullet_fire_timer(), game_time())
                    .with(color(), vec4(rng.gen_range(0.9..1.0), rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), 1.0))
                    .spawn();
            }
        }

        entity::set_component(client, angular_velocity(), vec3(0.0, 0.0, msg.player_movement * 0.6));

        let setting_velocity = movement_vector.extend(0.0) * msg.speed * 100.0;

        if msg.drifting == false && entity::has_component(client, game_player()) {
            entity::set_component(client, linear_velocity(), setting_velocity);
        }

        add_force_at_position(client, movement_vector.extend(0.0) * msg.speed * 1500.0, e_translation);

    });

    query(smoke_dissipation_delay()).requires(smoke_dissipation_delay()).each_frame(| bullet_fire | {
        for (smoke, delay) in bullet_fire {
            if entity::get_component(smoke, scale()).unwrap().z >= 0.0 {
                let bullet_scale: Option<Vec3> = entity::get_component(smoke, scale());
                entity::set_component(smoke, scale(), bullet_scale.unwrap() - delay);
            } else {
                entity::despawn(smoke);
            }
        }
    });
    

    query(bullet_movement()).requires(time_of_last_smoke_emission()).each_frame( | bullet | {
        for (bullet_id, mut bullet_velocity) in bullet {
            bullet_velocity.z = 0.02;
            entity::set_component(bullet_id, linear_velocity(), bullet_velocity * 20.0);
        }
    });

    query((translation(), smoke_trail_velocity())).requires(smoke_trail_velocity()).each_frame(| trail_smoke | {
        for (smoke_particle_id, (mut smoke_translation, smoke_directional_vector)) in trail_smoke {
            entity::set_component(smoke_particle_id, smoke_trail_velocity(), smoke_directional_vector * 0.95);
        }
    });

    query(track_spawn_time()).requires(track_spawn_time()).each_frame( | track | {
        for (track_id, track_time) in track {
            if game_time().as_secs_f32() - track_time.as_secs_f32() >= 20.0 {
                let track_scale = entity::get_component(track_id, scale()).unwrap();
                entity::set_component(track_id, scale(), track_scale - 0.05);
                if track_scale.x <= 0.0 {
                    entity::despawn(track_id);
                }
            }
        }
    });

    query((translation(), time_of_last_smoke_emission(), linear_velocity())).requires(time_of_last_smoke_emission()).each_frame(| bullet | {
        let mut rng = rand::thread_rng();
        for (bullet_id, (bullet_translation, last_smoke_emission, bullet_movement)) in bullet {
            // let smoke_spawn_location = bullet_translation + bullet_movement * -3.0;
            if game_time().as_secs_f32() - last_smoke_emission.as_secs_f32() >= 0.1 {
                Entity::new()
                    .with(cube(), ())
                    .with(translation(), bullet_translation + rng.gen_range(-0.5..0.5))
                    .with(smoke_dissipation_delay(), rng.gen_range(0.1..0.3) * 0.05)
                    .with(scale(), vec3(rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0)) * 0.3)
                    .with(rotation(), Quat::from_euler(EulerRot::XYZ, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0))
                    .with(smoke_trail_velocity(), bullet_movement)
                    .with(bullet_fire_timer(), game_time())
                    .with(color(), vec4(rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), 0.7))
                    .with(transparency_group(), 0)
                    .spawn();
                entity::set_component(bullet_id, time_of_last_smoke_emission(), game_time());
            }
        }
    });

    query((translation(), rotation(), time_between_treadmarks())).requires(game_player()).each_frame(| tank_data | {
        for (entity, (mut tank_translation, tank_rotation, time_between_last_placed_mark)) in tank_data {
            if game_time().as_secs_f32() - time_between_last_placed_mark.as_secs_f32() >= 0.1 {
                entity::set_component(entity, time_between_treadmarks(), game_time());
                tank_translation.z = -0.35;

                let tank_rotation_euler = tank_rotation.to_euler(EulerRot::XYZ);

                Bonk {
                    collision_sound_effect: 2,
                    emitter: entity,
                }.send_client_broadcast_unreliable();

                Entity::new()
                    .with(quad(), ())
                    .with(scale(), vec3(1.0, 2.0, 2.0))
                    .with(translation(), tank_translation)
                    .with(treadmark_display_timer(), game_time())
                    .with(rotation(), Quat::from_euler(EulerRot::XYZ, tank_rotation_euler.0, tank_rotation_euler.1, tank_rotation_euler.2 + std::f32::consts::PI/2.0))
                    .with(color(), vec4(1.0, 1.0, 1.0, 0.9))
                    .with(time_between_treadmarks(), game_time())
                    .with(transparency_group(), 0)
                    .with(track_spawn_time(), game_time())
                    .with(
                        pbr_material_from_url(),
                        packages::this::assets::url("pipeline.toml/2/mat.json"),
                    )
                    .spawn();
            }
        }
    });

}

fn spawn_cube(position: Vec3) -> EntityId {
    let entity = Entity::new()
        .with(translation(), position)
        .with(cube(), ())
        .with(scale(), Vec3::new(0.2, 0.2, 0.2));    
    entity.spawn()
}
fn reflect(v: &Vec3, n: &Vec3) -> Vec3 {
    *v - 2.0 * v.dot(*n) * *n
}