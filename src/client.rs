use ambient_api::{prelude::*, core::{messages::Frame, camera::{concepts::{PerspectiveInfiniteReverseCamera, PerspectiveInfiniteReverseCameraOptional}, components::{projection, active_camera, fog}}, transform::components::{lookat_target, translation, scale, rotation, lookat_up}, primitives::components::cube, rendering::components::{color, transparency_group}}, element::use_entity_component, glam::EulerRot, rand};
use packages::this::{messages::{KilledOther, SummonExplosion}, components::{kills_to_deaths, smoke_dissipation_delay, smoke_trail_velocity, bullet_fire_timer, tank_death_time, player_targetter_ref, game_player}};

use crate::packages::this::{messages::{Movement, Bonk, Respawn}, assets};

#[allow(unused_assignments)]

#[element_component]
fn PlayerPosition(hooks: &mut Hooks) -> Element {
    let kills_to_deaths = use_entity_component(hooks, player::get_local(), kills_to_deaths()).unwrap();
    let time_of_death = use_entity_component(hooks, player::get_local(), tank_death_time()).unwrap();
    let kd_text = "Kills: ".to_string() + &kills_to_deaths.x.to_string() + &" | Deaths: ".to_string() + &kills_to_deaths.y.to_string();
    let mut banner_message = "";
    if use_entity_component(hooks, player::get_local(), game_player()) == None {
        banner_message = "You are dead! Click 'R' to respawn! (Wait 5 seconds)";
    }
    FlowRow::el([
        Button::new(COLLECTION_ADD_ICON, move |_| {
            if game_time().as_millis() - time_of_death.as_millis() >= 5000 {
                Respawn {
                    sent_respawn_command: true
                }.send_server_reliable()
            }
        })
        .style(ButtonStyle::Card)
        .hotkey(VirtualKeyCode::R)
        .el(),
        Text::el(kd_text)
        .with(color(), vec4(1.0, 0.0, 0.0, 1.0)),
        Text::el(banner_message)
        .header_style(),
    ]).with_padding_even(2.0)
}

#[main]
pub fn main() {

    PlayerPosition.el().spawn_interactive();

    let mut movement = 0.0;

    let mut fire = false;

    let mut drifting = false;

    let mut speed: f32 = 0.0;

    let turret_rotation: i32 = 0;

    let camera = PerspectiveInfiniteReverseCamera {
        optional: PerspectiveInfiniteReverseCameraOptional {
            translation: Some(vec3(0., 0., 20.)),
            rotation: Some(Quat::default()),
            aspect_ratio_from_window: Some(entity::resources()),
            main_scene: Some(()),
            ..default()
        },
        ..PerspectiveInfiniteReverseCamera::suggested()
    }
    .make()
    .with(lookat_target(), Vec3::ZERO)
    .with(translation(), vec3(0.0, -0.1, 40.0))
    .with(active_camera(), 3.0)
    .with(rotation(), Quat::default())
    .spawn();

    let spatial_audio_player = audio::SpatialAudioPlayer::new();

    spatial_audio_player.set_amplitude(100.0);
    spatial_audio_player.set_listener(camera);

    Bonk::subscribe(move |_ctx, data| {
        if data.collision_sound_effect == 0 {
            spatial_audio_player.play_sound_on_entity(assets::url("bonk.ogg"), data.emitter);
        }
        else if data.collision_sound_effect == 1 {
            spatial_audio_player.play_sound_on_entity(assets::url("tankfire.ogg"), data.emitter);
        } else if data.collision_sound_effect == 2 {
            // spatial_audio_player.play(assets::url("tanktrack.ogg"));
        } else if data.collision_sound_effect == 3 {
            spatial_audio_player.play_sound_on_entity(assets::url("wall_bonk.ogg"), data.emitter);
        }

    });

    KilledOther::subscribe(move |_ctx, data| {
        if data.killed_other_player == true {
            let player = player::get_local();
            let current_kills_to_deaths = entity::get_component(player, kills_to_deaths()).unwrap();
            entity::set_component(player, kills_to_deaths(), vec2(current_kills_to_deaths.x, current_kills_to_deaths.y));
        }
    });

    SummonExplosion::subscribe(move |_ctx, data| {
        let mut rng = rand::thread_rng();
        for _i in 1..3 {
            Entity::new()
                .with(cube(), ())
                .with(translation(), data.translation + rng.gen_range(-1.0..1.0))
                .with(smoke_dissipation_delay(), rng.gen_range(0.1..0.3) * 0.1)
                .with(scale(), vec3(rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0)) * data.scale)
                .with(rotation(), Quat::from_euler(EulerRot::XYZ, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0))
                .with(smoke_trail_velocity(), Vec3::ZERO)
                .with(bullet_fire_timer(), game_time())
                .with(color(), vec4(rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), 0.7))
                .with(transparency_group(), 0)
                .spawn();
        }
        for _i in 1..3 {
            Entity::new()
                .with(cube(), ())
                .with(translation(), data.translation + rng.gen_range(-1.0..1.0))
                .with(smoke_dissipation_delay(), rng.gen_range(0.1..0.3) * 0.5)
                .with(scale(), vec3(rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0), rng.gen_range(1.8..2.0)) * data.scale)
                .with(rotation(), Quat::from_euler(EulerRot::XYZ, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0, rng.gen::<f32>() * 360.0))
                .with(smoke_trail_velocity(), Vec3::ZERO)
                .with(bullet_fire_timer(), game_time())
                .with(color(), vec4(rng.gen_range(0.9..1.0), rng.gen_range(0.1..0.2), rng.gen_range(0.1..0.2), 1.0))
                .spawn();
        }
    });

    Frame::subscribe(move |_| {

        let player_id = player::get_local();

        let player_translation = entity::get_component(player_id, translation()).unwrap();

        let player_rotation = entity::get_component(player_id, rotation()).unwrap();

        let e_rotation_nonvec = player_rotation.to_euler(EulerRot::XYZ);

        let e_rotation = vec3(e_rotation_nonvec.0, e_rotation_nonvec.1, e_rotation_nonvec.2);

        let movement_vector = vec2((e_rotation.z.to_degrees() + 90.0).to_radians().cos(), (e_rotation.z.to_degrees() + 90.0).to_radians().sin());

        let camera_position = player_translation - movement_vector.extend(0.0) * 20.0;

        entity::set_component(camera, translation(), vec3(player_translation.x, player_translation.y, 30.0));
        entity::set_component(camera, lookat_target(), player_translation + 0.1);
        // entity::set_component(camera, rotation(), player_rotation);
        // entity::set_component(camera, lookat_up(), movement_vector.extend(0.0));


        let (delta, input) = input::get_delta();

        if input.keys.contains(&KeyCode::D) && movement <= 3.0 {
            movement += 0.6;
        } if input.keys.contains(&KeyCode::A) && movement >= -3.0 {
            movement -= 0.6;
        }
        if movement < 0.0 && !input.keys.contains(&KeyCode::A) {
            movement += 0.1;
        }
        if movement > 0.0 && !input.keys.contains(&KeyCode::D){
            movement -= 0.1;
        }

        if input.keys.contains(&KeyCode::W) && speed <= 0.09 {
            speed += 0.003;
        } else if speed >= 0.0 {
            speed -= 0.003;
        }
        else if input.keys.contains(&KeyCode::S) && speed >= -0.06 {
            speed -= 0.002;
        } else if speed <= 0.0 {
            speed += 0.003;
        }

        if !delta.mouse_buttons.is_empty() {
            if delta.mouse_buttons.contains(&MouseButton::Left) {
                fire = true;
            } else {
                fire = false;
            }
        } else {
            fire = false;
        }

        if input.keys.contains(&KeyCode::LShift) {
            drifting = true;
        } else {
            drifting = false;
        }

        if !entity::has_component(camera, projection()) {
            // HACK: workaround for the orbit_camera package not adding components to the camera
            // entity until the next frame. In future, the API functions will be fallible, allowing them
            // to return an error if the entity doesn't have the required components.
            return;
        }

        let ray = camera::screen_position_to_world_ray(camera, input.mouse_position);

        // Send screen ray to server

        Movement {
            player_movement: movement,
            speed: speed,
            turret_turning: turret_rotation,
            ray_origin: ray.origin,
            ray_dir: ray.dir,
            fire: fire,
            drifting: drifting,
        }.send_server_unreliable();

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
    
}
