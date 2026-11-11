use std::{
    collections::{HashMap, HashSet},
    panic,
    sync::mpsc::channel,
};

#[cfg(not(target_arch = "wasm32"))]
use macroquad::file::set_pc_assets_folder;
use macroquad::{
    audio::{load_sound, play_sound_once},
    color::{BLACK, Color, WHITE},
    input::{KeyCode, MouseButton, is_key_pressed, is_mouse_button_pressed, mouse_position},
    math::{Rect, Vec2 as ScreenVec2, vec2},
    rand::gen_range,
    shapes::{draw_circle, draw_rectangle},
    text::{draw_text, measure_text},
    texture::{DrawTextureParams, FilterMode, Texture2D, draw_texture_ex, load_texture},
    time::get_frame_time,
    window::{Conf, clear_background, next_frame, screen_height, screen_width},
};
use rapier2d::{
    dynamics::{CoefficientCombineRule, RigidBodyBuilder, RigidBodyHandle},
    geometry::{ColliderBuilder, ColliderHandle, ColliderSet, CollisionEvent, TypedShape},
    math::{Vec2, Vector},
    pipeline::{ActiveEvents, ChannelEventCollector, PhysicsWorld},
};

// Game properties
const SPAWN_Y: f32 = 12.0;
const WORLD_WIDTH: f32 = 12.0;
const WORLD_HEIGHT: f32 = 14.0;
const WORLD_CENTER: Vec2 = Vec2::new(WORLD_WIDTH / 2.0, WORLD_HEIGHT / 2.0);
const GAME_OVER_CONTACT_DURATION: f32 = 1.0;

// Ball properties
const TEXTURE_HEIGHT: f32 = 512.0;
const TEXTURE_WIDTH: f32 = 512.0;
const RESTITUTION: f32 = 0.2;
const FRICTION: f32 = 0.8;

enum GameState {
    Playing,
    Paused,
    GameOver,
}

struct View {
    scale: f32,
    screen_center: ScreenVec2,
}

impl View {
    fn new() -> Self {
        let scale = (screen_width() / WORLD_WIDTH).min(screen_height() / WORLD_HEIGHT);
        Self {
            scale,
            screen_center: vec2(screen_width() / 2.0, screen_height() / 2.0),
        }
    }

    fn world_to_screen(&self, position: Vec2) -> ScreenVec2 {
        vec2(
            self.screen_center.x + (position.x - WORLD_CENTER.x) * self.scale,
            self.screen_center.y - (position.y - WORLD_CENTER.y) * self.scale,
        )
    }

    fn screen_to_world(&self, position: ScreenVec2) -> Vec2 {
        Vec2::new(
            (position.x - self.screen_center.x) / self.scale + WORLD_CENTER.x,
            (self.screen_center.y - position.y) / self.scale + WORLD_CENTER.y,
        )
    }

    fn update(&mut self) {
        self.scale = (screen_width() / WORLD_WIDTH).min(screen_height() / WORLD_HEIGHT);
        self.screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
    }
}

struct Ball {
    level: usize,
    radius: f32,
    body: RigidBodyHandle,
    collider: ColliderHandle,
}

impl Ball {
    fn new(
        level: usize,
        radius: f32,
        body_handle: RigidBodyHandle,
        collider_handle: ColliderHandle,
    ) -> Self {
        Ball {
            level: level,
            radius: radius,
            body: body_handle,
            collider: collider_handle,
        }
    }

    fn draw(&self, collider_set: &ColliderSet, view: &View, sprite_sheet: &Texture2D) {
        if let Some(collider) = collider_set.get(self.collider) {
            let center = collider.position().translation;
            let screen_center = view.world_to_screen(center);
            let diameter = self.radius * 2.0 * view.scale;

            draw_texture_ex(
                sprite_sheet,
                screen_center.x - diameter / 2.0,
                screen_center.y - diameter / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(diameter, diameter)),
                    rotation: -collider.position().rotation.angle(),
                    source: Some(Rect::new(
                        0.0,
                        self.level as f32 * TEXTURE_HEIGHT,
                        TEXTURE_WIDTH,
                        TEXTURE_HEIGHT,
                    )),
                    ..Default::default()
                },
            );
        }
    }
}

fn create_ball(
    world: &mut PhysicsWorld,
    level: usize,
    radius: f32,
    position: Vec2,
    enabled: bool,
) -> Ball {
    let (body_handle, collider_handle) = world.insert(
        RigidBodyBuilder::dynamic().translation(position),
        ColliderBuilder::ball(radius)
            .restitution(RESTITUTION)
            .friction(FRICTION)
            .friction_combine_rule(CoefficientCombineRule::Multiply)
            .enabled(enabled)
            .active_events(ActiveEvents::COLLISION_EVENTS),
    );

    Ball::new(level, radius, body_handle, collider_handle)
}

fn draw_colliders(collider_set: &ColliderSet, handles: &[ColliderHandle], view: &View) {
    for handle in handles {
        let Some(collider) = collider_set.get(*handle) else {
            continue;
        };
        let center = collider.position().translation;
        let screen_center = view.world_to_screen(center);

        match collider.shape().as_typed_shape() {
            TypedShape::Cuboid(cuboid) => {
                let half_extends = cuboid.half_extents * view.scale;
                draw_rectangle(
                    screen_center.x - half_extends.x,
                    screen_center.y - half_extends.y,
                    half_extends.x * 2.0,
                    half_extends.y * 2.0,
                    BLACK,
                );
            }
            TypedShape::Ball(ball) => {
                draw_circle(
                    screen_center.x,
                    screen_center.y,
                    ball.radius * view.scale,
                    BLACK,
                );
            }
            _ => {
                panic!("Unknown shape")
            }
        }
    }
}

fn draw_score(score: usize) {
    let score_text = format!("Score : {}", score);
    let font_size = 24.0;
    let text_width = measure_text(&score_text, None, font_size as u16, 1.0).width;
    draw_text(
        &score_text,
        screen_width() - text_width - 10.0,
        font_size + 10.0,
        font_size,
        BLACK,
    );
}

fn draw_state_overlay(state: &GameState, score: usize) {
    let (title, score, hint) = match state {
        GameState::Playing => return,
        GameState::Paused => (
            "PAUSE",
            &format!("Score : {}", score),
            "ESC : resume    R : restart",
        ),
        GameState::GameOver => ("GAME OVER", &format!("Score : {}", score), "R : restart"),
    };
    let width = screen_width();
    let height = screen_height();

    draw_rectangle(0.0, 0.0, width, height, Color::new(0.0, 0.0, 0.0, 0.55));
    let title_size = 64.0;
    let hint_size = 24.0;
    let title_width = measure_text(title, None, title_size as u16, 1.0).width;
    let score_width = measure_text(score, None, title_size as u16 / 2, 1.0).width;
    let hint_width = measure_text(hint, None, hint_size as u16, 1.0).width;

    draw_text(
        title,
        (width - title_width) / 2.0,
        height / 2.0,
        title_size,
        WHITE,
    );
    draw_text(
        score,
        (width - score_width) / 2.0,
        height / 2.0 + 32.0,
        title_size / 2.0,
        WHITE,
    );
    draw_text(
        hint,
        (width - hint_width) / 2.0,
        height / 2.0 + 64.0,
        hint_size,
        WHITE,
    );
}

fn window_conf() -> Conf {
    Conf {
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    set_pc_assets_folder("web");

    // Loading screen
    clear_background(WHITE);
    let text = "Chargement...";
    let font_size = 40.0;
    let text_width = measure_text(text, None, font_size as u16, 1.0).width;
    draw_text(
        text,
        (screen_width() - text_width) / 2.0,
        screen_height() / 2.0,
        font_size,
        BLACK,
    );
    next_frame().await;

    // Load sound effects
    let sound_drop = load_sound("assets/pop.mp3")
        .await
        .expect("Failed to load sound effect");

    // Load ball properties
    let sprite_sheet = load_texture("assets/sprite_sheet.webp")
        .await
        .expect("Failed to load sprite sheet");
    sprite_sheet.set_filter(FilterMode::Linear);
    let ball_radius = vec![0.2, 0.4, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.4, 1.6];

    // Game state inits
    // Score
    let mut score = 0;
    // Balls dropped by the player
    let mut balls: Vec<Ball> = Vec::new();
    // Ball collider -> level
    let mut ball_level_by_collider = HashMap::new();
    // Balls over height limit -> for how long
    let mut sensor_contacts = HashMap::new();
    // Util to center the game inside the window
    let mut view = View::new();
    // Physics engine
    let mut world = PhysicsWorld::new();
    world.gravity = Vector::new(0.0, -9.81);

    // Collision channels
    let (collision_send, collision_recv) = channel();
    let event_handler = ChannelEventCollector::new(collision_send, channel().0, channel().0);

    // Environment colliders
    let basket = vec![
        world.insert_collider(
            ColliderBuilder::cuboid(4.0, 0.5)
                .translation(Vec2::new(6.0, 1.0))
                .friction(FRICTION),
            None,
        ),
        world.insert_collider(
            ColliderBuilder::cuboid(0.5, 6.0)
                .translation(Vec2::new(2.5, 6.5))
                .friction(FRICTION),
            None,
        ),
        world.insert_collider(
            ColliderBuilder::cuboid(0.5, 6.0)
                .translation(Vec2::new(9.5, 6.5))
                .friction(FRICTION),
            None,
        ),
    ];
    let height_sensor = world.insert_collider(
        ColliderBuilder::cuboid(4.0, 0.1)
            .translation(Vec2::new(6.0, SPAWN_Y - 1.0))
            .sensor(true)
            .active_events(ActiveEvents::COLLISION_EVENTS),
        None,
    );

    // New ball to be dropped
    let mut level: usize = gen_range(0, 4);
    let mut radius = ball_radius[level];
    let mut new_ball_x = view
        .screen_to_world(mouse_position().into())
        .x
        .clamp(3.0 + radius, 9.0 - radius);
    let mut new_ball = create_ball(
        &mut world,
        level,
        radius,
        Vec2::new(new_ball_x, SPAWN_Y),
        false,
    );

    // Game loop
    let mut game_state = GameState::Playing;
    loop {
        view.update();

        if is_key_pressed(KeyCode::Escape) {
            match game_state {
                GameState::Playing => game_state = GameState::Paused,
                GameState::Paused => game_state = GameState::Playing,
                GameState::GameOver => {}
            }
        }

        if matches!(game_state, GameState::Paused | GameState::GameOver)
            && is_key_pressed(KeyCode::R)
        {
            // Reset game
            score = 0;
            for ball in balls.drain(..) {
                world.remove_body_with_colliders(ball.body, true);
            }
            world.remove_body_with_colliders(new_ball.body, true);
            ball_level_by_collider.clear();
            sensor_contacts.clear();

            // New ball
            level = gen_range(0, 4);
            radius = ball_radius[level];
            new_ball_x = view
                .screen_to_world(mouse_position().into())
                .x
                .clamp(3.0 + radius, 9.0 - radius);
            new_ball = create_ball(
                &mut world,
                level,
                radius,
                Vec2::new(new_ball_x, SPAWN_Y),
                false,
            );

            // Drain previous game collision messages
            while collision_recv.try_recv().is_ok() {}

            game_state = GameState::Playing;
        }

        if let GameState::Playing = game_state {
            // Drop new ball
            if is_mouse_button_pressed(MouseButton::Left) {
                // Enable physics on new ball
                if let Some(collider) = world.colliders.get_mut(new_ball.collider) {
                    collider.set_enabled(true);
                }
                if let Some(body) = world.bodies.get_mut(new_ball.body) {
                    body.wake_up(true);
                }

                // Add to dropped balls
                ball_level_by_collider.insert(new_ball.collider, level);
                balls.push(new_ball);

                // Update score
                score += level;

                // New ball to be dropped
                level = gen_range(0, 4);
                radius = ball_radius[level];
                new_ball = create_ball(
                    &mut world,
                    level,
                    radius,
                    Vec2::new(new_ball_x, SPAWN_Y),
                    false,
                );
            }

            // Make new ball follow the mouse horizontally
            new_ball_x = view
                .screen_to_world(mouse_position().into())
                .x
                .clamp(3.0 + radius, 9.0 - radius);
            if let Some(body) = world.bodies.get_mut(new_ball.body) {
                body.set_translation(Vec2::new(new_ball_x, SPAWN_Y), true);
            }

            // Physics engine step
            world.step_with_events(&(), &event_handler);

            // Fusions to process in this step
            let mut fusions = Vec::new();
            // Colliders that have been merged (to be deleted)
            let mut merged_colliders = HashSet::new();

            // Iterate on all received collision events
            while let Ok(event) = collision_recv.try_recv() {
                let (handle_a, handle_b) = match event {
                    CollisionEvent::Started(handle_a, handle_b, _) => (handle_a, handle_b),
                    CollisionEvent::Stopped(handle_a, handle_b, _) => {
                        // The only collision that can stop are with the height sensor (other are processed in the same frame)
                        let ball_handle = if handle_a == height_sensor {
                            Some(handle_b)
                        } else if handle_b == height_sensor {
                            Some(handle_a)
                        } else {
                            None
                        };
                        if let Some(ball_handle) = ball_handle {
                            sensor_contacts.remove(&ball_handle);
                        }
                        continue;
                    }
                };

                // Handle a collision with the height sensor
                let ball_handle = if handle_a == height_sensor {
                    Some(handle_b)
                } else if handle_b == height_sensor {
                    Some(handle_a)
                } else {
                    None
                };
                if let Some(ball_handle) = ball_handle
                    && ball_level_by_collider.contains_key(&ball_handle)
                {
                    sensor_contacts.entry(ball_handle).or_insert(0.0);
                }

                // Handle fusion between two balls
                let Some(level_a) = ball_level_by_collider.get(&handle_a).copied() else {
                    continue;
                };
                let Some(level_b) = ball_level_by_collider.get(&handle_b).copied() else {
                    continue;
                };
                if level_a == level_b
                    && level_a + 1 < ball_radius.len()
                    && !merged_colliders.contains(&handle_a)
                    && !merged_colliders.contains(&handle_b)
                    && balls.iter().any(|ball| ball.collider == handle_a)
                    && balls.iter().any(|ball| ball.collider == handle_b)
                {
                    merged_colliders.insert(handle_a);
                    merged_colliders.insert(handle_b);
                    fusions.push((handle_a, handle_b, level_a));
                }
            }

            // Compute contact duration with height sensor
            let frame_time = get_frame_time();
            for duration in sensor_contacts.values_mut() {
                *duration += frame_time;
            }
            // Check if game over
            if sensor_contacts
                .values()
                .any(|duration| *duration >= GAME_OVER_CONTACT_DURATION)
            {
                game_state = GameState::GameOver;
            }

            // Apply ball fusions
            for (handle_a, handle_b, level) in fusions {
                let Some(index_a) = balls.iter().position(|ball| ball.collider == handle_a) else {
                    continue;
                };
                let Some(index_b) = balls.iter().position(|ball| ball.collider == handle_b) else {
                    continue;
                };

                let position_a = world
                    .colliders
                    .get(handle_a)
                    .map(|collider| collider.position().translation);
                let position_b = world
                    .colliders
                    .get(handle_b)
                    .map(|collider| collider.position().translation);
                let (Some(position_a), Some(position_b)) = (position_a, position_b) else {
                    continue;
                };

                let midpoint = (position_a + position_b) / 2.0;
                let body_a = balls[index_a].body;
                let body_b = balls[index_b].body;

                // Delete fused balls from everything
                world.remove_body_with_colliders(body_a, true);
                world.remove_body_with_colliders(body_b, true);
                ball_level_by_collider.remove(&handle_a);
                ball_level_by_collider.remove(&handle_b);
                sensor_contacts.remove(&handle_a);
                sensor_contacts.remove(&handle_b);
                balls.swap_remove(index_a.max(index_b));
                balls.swap_remove(index_a.min(index_b));

                // Create new ball with next level
                let next_level = level + 1;
                let next_radius = ball_radius[next_level];
                let next_ball = create_ball(&mut world, next_level, next_radius, midpoint, true);
                ball_level_by_collider.insert(next_ball.collider, next_level);
                balls.push(next_ball);

                // Update score
                score += 1 << next_level;

                // Play pop sound
                play_sound_once(&sound_drop);
            }
        }

        clear_background(WHITE);

        draw_colliders(&world.colliders, &basket, &view);
        draw_colliders(&world.colliders, &[height_sensor], &view);

        draw_score(score);

        balls
            .iter()
            .for_each(|ball| ball.draw(&world.colliders, &view, &sprite_sheet));

        new_ball.draw(&world.colliders, &view, &sprite_sheet);
        draw_state_overlay(&game_state, score);

        next_frame().await;
    }
}
