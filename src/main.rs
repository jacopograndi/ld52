use rand::{rngs::ThreadRng, Rng};
use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_asset_loader::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use bevy_rapier2d::prelude::*;
use bevy_vfx_bag::{
    image::chromatic_aberration::ChromaticAberrationPlugin, BevyVfxBagPlugin, PostProcessingInput,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: 700.0,
                height: 700.0,
                fit_canvas_to_parent: true,
                ..default()
            },
            ..default()
        }))
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::LevelLoading)
                .with_collection::<GameAssets>(),
        )
        .add_state(GameState::AssetLoading)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        //.add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_egui)
        .add_startup_system(setup_physics)
        .add_system_set(SystemSet::on_update(GameState::LevelLoading).with_system(setup_level))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(animate_sprite))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(block_color))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(movement))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(check_finish))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(show_level_progress))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(mass_increase))
        .add_system_set(SystemSet::on_exit(GameState::Gameplay).with_system(teardown_level))
        .add_system(mouse_pos)
        .add_plugin(EguiPlugin)
        .insert_resource(Msaa { samples: 1 })
        .init_resource::<Info>()
        .init_resource::<MousePos>()
        .insert_resource(Progress {
            current_level: 1,
            golden_apples: 0,
            level_complete: false,
            end_timer: Timer::from_seconds(5.0, TimerMode::Once),
            level_timer: Timer::from_seconds(20.0, TimerMode::Once),
            num: 1,
        })
        .insert_resource(ClearColor(Color::BLACK))
        //.add_plugin(BevyVfxBagPlugin) // This needs to be added for any effect to work
        //.add_plugin(ChromaticAberrationPlugin)
        .run();
}

#[derive(AssetCollection, Resource)]
struct GameAssets {
    #[asset(texture_atlas(tile_size_x = 64., tile_size_y = 64., columns = 4, rows = 1))]
    #[asset(path = "sheet.png")]
    lolle: Handle<TextureAtlas>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    LevelLoading,
    Gameplay,
}

fn setup_graphics(mut commands: Commands) {
    commands
        .spawn(Camera2dBundle::default())
        .insert(PostProcessingInput);
}

fn place_block(commands: &mut Commands, block: Block) {
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: block.base_color * 0.5,
                custom_size: Some(block.base_size),
                ..default()
            },
            ..default()
        })
        .insert(BlockBase)
        .insert(TransformBundle::from(Transform::from_translation(
            block.base_pos - Vec3::new(0.0, 0.0, 5.0),
        )));

    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: block.color,
                custom_size: Some(block.base_size),
                ..default()
            },
            ..default()
        })
        .insert(block.clone())
        .insert(TransformBundle::from(Transform::from_translation(
            block.base_pos,
        )))
        .insert(Collider::cuboid(
            block.base_size.x * 0.5,
            block.base_size.y * 0.5,
        ))
        .insert(Damping {
            linear_damping: block.linear_damping,
            angular_damping: block.angular_damping,
        })
        .insert(Restitution::coefficient(block.restitution))
        .insert(ColliderMassProperties::Density(block.density))
        .insert(RigidBody::Dynamic);
}

fn setup_physics(mut commands: Commands, mut rapier_conf: ResMut<RapierConfiguration>) {
    rapier_conf.gravity = Vec2::new(0.0, 0.0);
    commands
        .spawn(Collider::cuboid(700.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 340.0, 0.0)));

    commands
        .spawn(Collider::cuboid(700.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -340.0, 0.0)));

    commands
        .spawn(Collider::cuboid(50.0, 700.0))
        .insert(TransformBundle::from(Transform::from_xyz(-400.0, 0.0, 0.0)));

    commands
        .spawn(Collider::cuboid(50.0, 700.0))
        .insert(TransformBundle::from(Transform::from_xyz(400.0, 0.0, 0.0)));
}

fn setup_egui(mut egui_context: ResMut<EguiContext>, mut egui_settings: ResMut<EguiSettings>) {
    let ctx = egui_context.ctx_mut();
    let mut style: egui::Style = (*ctx.style()).clone();
    style.visuals.window_fill = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120);
    style.visuals.window_rounding = egui::Rounding::none();
    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.override_text_color = Some(egui::Color32::WHITE);
    style.spacing.item_spacing = egui::vec2(10.0, 20.0);
    ctx.set_style(style);

    egui_settings.scale_factor = 2.0;
}

fn setup_level(
    mut commands: Commands,
    my_assets: Res<GameAssets>,
    mut info: ResMut<Info>,
    prog: Res<Progress>,
    mut bg_color: ResMut<ClearColor>,
    mut state: ResMut<State<GameState>>,
) {
    let level = info.get_level(&prog);
    for block in level.blocks.iter() {
        place_block(&mut commands, block.clone());
    }

    bg_color.0 = level.back_color.clone();

    commands
        .spawn(SpriteSheetBundle {
            sprite: TextureAtlasSprite::new(0),
            texture_atlas: my_assets.lolle.clone(),
            ..Default::default()
        })
        .insert(AnimationTimer(Timer::from_seconds(
            0.25,
            TimerMode::Repeating,
        )))
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball(30.0))
        .insert(Restitution::coefficient(0.7))
        .insert(Damping {
            linear_damping: 5.0,
            angular_damping: 1.0,
        })
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(Velocity {
            linvel: Vec2::new(0.0, 0.0),
            ..Default::default()
        })
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(ActiveEvents::CONTACT_FORCE_EVENTS)
        .insert(AnimationState::Walking)
        .insert(MainCharacter {
            dash_timer: Timer::from_seconds(1.0, TimerMode::Once),
        })
        .insert(TransformBundle::from(Transform::from_translation(
            level.spawnpoint,
        )));

    state.set(GameState::Gameplay).ok();
}

fn teardown_level(
    mut commands: Commands,
    query_blocks: Query<(Entity, &Block)>,
    query_main: Query<(Entity, &MainCharacter)>,
    query_bases: Query<(Entity, &BlockBase)>,
) {
    for (ent, _) in query_blocks.iter() {
        commands.entity(ent).despawn();
    }
    for (ent, _) in query_main.iter() {
        commands.entity(ent).despawn();
    }
    for (ent, _) in query_bases.iter() {
        commands.entity(ent).despawn();
    }
}

fn mass_increase(
    mut main_char: Query<(&mut MainCharacter, &mut AdditionalMassProperties)>,
    block_query: Query<(&Block, &Transform)>,
    prog: Res<Progress>,
) {
    if let Ok((_, mut mass)) = main_char.get_single_mut() {
        let sum: f32 = block_query
            .iter()
            .map(|(block, tr)| block.rel(tr.translation))
            .sum();
        *mass = AdditionalMassProperties::Mass(1.0 + sum * 2000.0);
    }
}

fn check_finish(
    mut state: ResMut<State<GameState>>,
    block_query: Query<(&Block, &Transform)>,
    mut info: ResMut<Info>,
    mut prog: ResMut<Progress>,
    time: Res<Time>,
) {
    if block_query.iter().count() == 0 {
        return;
    }
    if !prog.level_complete {
        let sum: f32 = block_query
            .iter()
            .map(|(block, tr)| block.rel(tr.translation))
            .sum();
        let level = info.get_level(&prog);
        prog.level_timer.tick(time.delta());
        if sum >= level.point_threshold || prog.level_timer.finished() {
            prog.num += 1;
            if !prog.level_timer.finished() {
                prog.golden_apples += 1;
            }
            prog.level_complete = true;
            prog.end_timer.reset();
            /*
            let next_level = (info
                .levels
                .iter()
                .enumerate()
                .find(|l| l.1.id == prog.current_level)
                .unwrap()
                .0
                + 1)
                % info.levels.len();
            */
            prog.current_level += 1;
        }
    } else {
        prog.end_timer.tick(time.delta());
        if prog.end_timer.finished() {
            let level = info.get_level(&prog);
            prog.level_timer.set_duration(level.duration);
            prog.level_timer.reset();
            prog.level_complete = false;
            state.set(GameState::LevelLoading).ok();
        }
    }
}

fn show_level_progress(
    block_query: Query<(&Block, &Transform)>,
    mut info: ResMut<Info>,
    prog: Res<Progress>,
    mut egui_context: ResMut<EguiContext>,
) {
    let sum: f32 = block_query
        .iter()
        .map(|(block, tr)| block.rel(tr.translation))
        .sum();
    let level = info.get_level(&prog);
    let progress = sum / level.point_threshold;
    let col = level.color.as_rgba();
    let color = egui::Color32::from_rgb(
        (col.r() * 255.0) as i32 as u8,
        (col.g() * 255.0) as i32 as u8,
        (col.b() * 255.0) as i32 as u8,
    );
    egui::Window::new("LevelProgress")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::splat(0.0))
        .show(egui_context.ctx_mut(), |ui| {
            ui.visuals_mut().selection.bg_fill = color;
            if progress < 1.0 && !prog.level_complete {
                let progress_bar = egui::ProgressBar::new(progress)
                    .show_percentage()
                    .text(format!("{:.0}% of level {}", progress * 100.0, prog.num));
                ui.add(progress_bar);
            } else {
                if !prog.level_timer.finished() {
                    ui.label("You won a golden apple!");
                } else {
                    ui.label("Not fast enough!");
                }
                ui.label(format!("You have {} golden apples.", prog.golden_apples));
            }
        });
    egui::Window::new("TimeProgress")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::Vec2::splat(0.0))
        .show(egui_context.ctx_mut(), |ui| {
            ui.visuals_mut().selection.bg_fill = color;

            let progress_bar =
                egui::ProgressBar::new(1.0 - prog.level_timer.percent()).text("Time left");
            ui.add(progress_bar);
        });
}

fn block_color(mut block_query: Query<(&Block, &Transform, &mut Sprite)>) {
    for (block, tr, mut sprite) in block_query.iter_mut() {
        let amt = block.rel(tr.translation);
        let lerp = block.color * (1.0 - amt) + block.color_target * amt;
        sprite.color = lerp;
    }
}

#[derive(Clone)]
struct Level {
    id: u32,
    blocks: Vec<Block>,
    point_threshold: f32,
    spawnpoint: Vec3,
    duration: Duration,
    back_color: Color,
    color: Color,
}

#[derive(Clone)]
struct Rect {
    pos: Vec2,
    size: Vec2,
}

#[derive(Clone)]
struct Layout {
    rects: Vec<Rect>,
}

impl Layout {
    fn gen(mag: u32, mut rng: &mut ThreadRng) -> Layout {
        let mut layout = Layout { rects: vec![] };
        for i in 0..mag {
            // generate random box size
            let size = Vec2::new(rng.gen_range(20.0..60.0), rng.gen_range(20.0..60.0));

            // select a direction (along x or y)
            let dir = if rng.gen_bool(0.5) {
                Vec2::new(1.0, 0.0)
            } else {
                Vec2::new(0.0, 1.0)
            };

            // generate a random offset from 0..max * 2
            let mut maxvec = Vec2::new(0.0, 0.0);
            for r in layout.rects.iter() {
                maxvec.x = maxvec.x.max(r.pos.x + r.size.x);
                maxvec.y = maxvec.y.max(r.pos.y + r.size.y);
            }
            let max = (maxvec * dir).length();
            let off = dir
                * if max > 0.0 {
                    rng.gen_range(0.0..max)
                } else {
                    0.0
                };

            // move the box along other dir to not overlap the others
            let rect = Rect {
                pos: off * dir,
                size,
            };
            let casted = layout.cast(&(dir * off), &dir, &rect);

            layout.rects.push(casted);

            // mirror along x and y
        }
        layout
    }

    fn intersects(&self, rect: &Rect) -> bool {
        self.rects.iter().any(|r| {
            r.pos.x < rect.pos.x + rect.size.x
                && rect.size.x < r.pos.x + r.size.x
                && r.pos.y < rect.pos.y + rect.size.y
                && rect.size.y < r.pos.y + r.size.y
        })
    }

    fn cast(&self, start: &Vec2, dir: &Vec2, rect: &Rect) -> Rect {
        let mut pos = start.clone() + (dir.clone() * (self.rects.len() as f32 * 50.0));
        for r in self.rects.iter() {
            //r.pos * dir
        }
        Rect {
            pos,
            size: rect.size.clone(),
        }
    }
}

impl Level {
    fn gen(num: u32) -> Level {
        let mut rng = rand::thread_rng();
        let hue: f32 = rng.gen_range(0.0..360.0);
        let color = Color::hsl(hue, 1.0, 0.5);
        let target_color = Color::hsl((hue + 137.0).clamp(0.0, 360.0), 1.0, 0.5);
        let back_color = Color::hsl(0.0, 0.0, 0.1);
        let movable = Block {
            color,
            base_size: Vec2::new(30.0, 30.0),
            base_color: color * 0.7,
            color_target: target_color,
            ..default()
        };
        let mag = match num {
            0..=4 => 4,
            n => n,
        };
        let layout = Layout::gen(mag, &mut rng);
        let mut blocks: Vec<Block> = layout
            .rects
            .iter()
            .map(|r| movable.mov(r.pos).siz(r.size))
            .collect();
        Level {
            id: num,
            back_color,
            blocks: blocks.clone(),
            point_threshold: blocks.iter().filter(|b| b.density > 0.0).count() as f32 * 0.8,
            spawnpoint: Vec3::new(0.0, 0.0, 20.0),
            duration: Duration::from_secs(20),
            color,
        }
    }
}

#[derive(Resource)]
struct Info {
    levels: Vec<Level>,
}

impl Default for Info {
    fn default() -> Self {
        Self { levels: vec![] }
    }
}

impl Info {
    fn get_level(&mut self, prog: &Progress) -> Level {
        if let Some(level) = self.levels.iter().find(|l| l.id == prog.current_level) {
            level.clone()
        } else {
            let level = Level::gen(prog.current_level);
            self.levels.push(level.clone());
            level
        }
    }
}

#[derive(Resource)]
struct Progress {
    current_level: u32,
    golden_apples: i32,
    level_complete: bool,
    end_timer: Timer,
    level_timer: Timer,
    num: u32,
}

#[derive(Component)]
struct BlockBase;

#[derive(Component, Clone)]
struct Block {
    base_pos: Vec3,
    base_color: Color,
    base_size: Vec2,
    color: Color,
    color_target: Color,
    max_distance: f32,
    points_mul: f32,
    linear_damping: f32,
    angular_damping: f32,
    restitution: f32,
    density: f32,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            base_pos: Vec3::new(0.0, 0.0, 0.0),
            base_color: Color::PURPLE,
            base_size: Vec2::new(40.0, 40.0),
            color: Color::PURPLE,
            max_distance: 300.0,
            linear_damping: 1.0,
            angular_damping: 1.0,
            restitution: 0.3,
            density: 1.0,
            points_mul: 1.0,
            color_target: Color::RED,
        }
    }
}

impl Block {
    fn rel(&self, pos: Vec3) -> f32 {
        let distance = pos.distance(self.base_pos);
        let amt = distance / self.max_distance;
        let amt = amt.max(0.0).min(1.0);
        amt
    }

    fn mov(&self, pos: Vec2) -> Block {
        let mut moved = self.clone();
        moved.base_pos = Vec3::new(pos.x, pos.y, 10.0);
        moved
    }

    fn siz(&self, size: Vec2) -> Block {
        let mut moved = self.clone();
        moved.base_size = size;
        moved
    }
}

#[derive(Component)]
enum AnimationState {
    Walking,
    Dashing,
}

#[derive(Component)]
struct MainCharacter {
    dash_timer: Timer,
}

#[derive(Component)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &AnimationState,
    )>,
    main_query: Query<(&Velocity, &MainCharacter)>,
) {
    if let Ok((vel, _)) = main_query.get_single() {
        for (mut timer, mut sprite, anim) in &mut query {
            let speed = vel.linvel.length().min(1.0);
            timer.0.tick(time.delta() * speed as u32);
            if timer.0.finished() {
                match anim {
                    AnimationState::Walking => {
                        sprite.index = (sprite.index + 1) % 4;
                    }
                    _ => (),
                }
            }
        }
    }
}

const ACCELERATION: f32 = 50.0;
const DASH_BOOST: f32 = 2.5;

fn movement(
    mut main_char: Query<(&mut MainCharacter, &Transform, &mut Velocity)>,
    mut touch_input: EventReader<TouchInput>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mouse_pos: Res<MousePos>,
    time: Res<Time>,
    prog: Res<Progress>,
) {
    if let Ok((mut main, tr, mut vel)) = main_char.get_single_mut() {
        let pos = Vec2::new(tr.translation.x, tr.translation.y);
        let mut acc = ACCELERATION;
        main.dash_timer.tick(time.delta());
        if main.dash_timer.finished() {
            if keyboard_input.pressed(KeyCode::Space) {
                main.dash_timer.reset();
                acc *= DASH_BOOST * 2.0;
            }
        } else if main.dash_timer.percent() < 0.25 {
            acc *= DASH_BOOST;
        }
        let mut vec_acc = Vec2::new(
            keyboard_input.pressed(KeyCode::D) as i32 as f32
                - keyboard_input.pressed(KeyCode::A) as i32 as f32,
            keyboard_input.pressed(KeyCode::W) as i32 as f32
                - keyboard_input.pressed(KeyCode::S) as i32 as f32,
        );
        if mouse_input.pressed(MouseButton::Left) {
            vec_acc += (mouse_pos.world - pos).normalize_or_zero();
        }
        for touch in touch_input.iter() {
            vec_acc += (touch.position - pos).normalize_or_zero();
        }
        vel.linvel += vec_acc.normalize_or_zero() * acc * time.delta_seconds() * 60.0;
    }
}

#[derive(Resource, Default)]
struct MousePos {
    world: Vec2,
}

fn mouse_pos(
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Res<Windows>,
    mut mousepos: ResMut<MousePos>,
    query_camera: Query<(&Camera, &GlobalTransform)>,
) {
    if let Some((camera, camera_transform)) = query_camera.get_single().ok() {
        if let Some(window) = windows.get_primary() {
            for event in cursor_moved_events.iter() {
                let window_size = Vec2::new(window.width() as f32, window.height() as f32);
                let ndc = (event.position / window_size) * 2.0 - Vec2::ONE;
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.projection_matrix().inverse();
                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
                let world_pos: Vec2 = world_pos.truncate();
                mousepos.world = world_pos;
            }
        }
    }
}
