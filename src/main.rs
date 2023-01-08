use rand::{rngs::ThreadRng, Rng};
use std::time::Duration;

use bevy::{
    audio::AudioSink,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_asset_loader::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use bevy_rapier2d::prelude::*;
use bevy_vfx_bag::{
    image::chromatic_aberration::{ChromaticAberration, ChromaticAberrationPlugin},
    BevyVfxBagPlugin, PostProcessingInput,
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
        .insert_resource(Msaa { samples: 1 })
        .init_resource::<Info>()
        .init_resource::<MousePos>()
        .init_resource::<Soundtrack>()
        .insert_resource(Progress {
            current_level: 1,
            golden_apples: 0,
            level_complete: false,
            end_timer: Timer::from_seconds(3.0, TimerMode::Once),
            level_timer: Timer::from_seconds(20.0, TimerMode::Once),
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugin(BevyVfxBagPlugin) // This needs to be added for any effect to work
        .add_plugin(ChromaticAberrationPlugin)
        .insert_resource(ChromaticAberration {
            magnitude_r: 0.0,
            magnitude_g: 0.0,
            magnitude_b: 0.0,
            ..default()
        })
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        //.add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_egui)
        .add_startup_system(setup_physics)
        //.add_system_set(SystemSet::on_exit(GameState::AssetLoading).with_system(setup_audio))
        .add_system_set(SystemSet::on_update(GameState::LevelLoading).with_system(setup_level))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(animate_sprite))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(block_color))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(movement))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(check_finish))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(show_level_progress))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(mass_increase))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(attract))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(explode))
        .add_system_set(SystemSet::on_update(GameState::Gameplay).with_system(audio_volumes))
        .add_system_set(SystemSet::on_exit(GameState::Gameplay).with_system(teardown_level))
        .add_system(mouse_pos)
        .add_system(reset_chroma)
        .add_plugin(EguiPlugin)
        .run();
}

fn reset_chroma(mut chroma: ResMut<ChromaticAberration>, time: Res<Time>) {
    chroma.magnitude_r *= 0.8;
    chroma.magnitude_g *= 0.8;
    chroma.magnitude_b *= 0.8;
}

#[derive(AssetCollection, Resource)]
struct GameAssets {
    #[asset(texture_atlas(tile_size_x = 64., tile_size_y = 64., columns = 4, rows = 4))]
    #[asset(path = "sheet.png")]
    lolle: Handle<TextureAtlas>,
    #[asset(path = "hi.ogg")]
    hi: Handle<AudioSource>,
    #[asset(path = "lo.ogg")]
    lo: Handle<AudioSource>,
    #[asset(path = "hit.ogg")]
    hit: Handle<AudioSource>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    LevelLoading,
    Gameplay,
}

fn setup_graphics(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.projection.scale = 0.666;
    commands.spawn(camera).insert(PostProcessingInput);
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
        .insert(ActiveEvents::CONTACT_FORCE_EVENTS)
        .insert(Velocity {
            linvel: Vec2::ZERO,
            angvel: 0.0,
        })
        .insert(RigidBody::Dynamic);
}

#[derive(Resource, Default)]
struct Soundtrack {
    hi: Handle<AudioSink>,
    lo: Handle<AudioSink>,
}

fn setup_audio(
    my_assets: Res<GameAssets>,
    audio_sinks: Res<Assets<AudioSink>>,
    audio: Res<Audio>,
    mut sinks: ResMut<Soundtrack>,
) {
    sinks.hi = audio_sinks.get_handle(audio.play_with_settings(
        my_assets.hi.clone(),
        PlaybackSettings {
            repeat: true,
            volume: 0.1,
            ..default()
        },
    ));
    sinks.lo = audio_sinks.get_handle(audio.play_with_settings(
        my_assets.lo.clone(),
        PlaybackSettings {
            repeat: true,
            volume: 0.1,
            ..default()
        },
    ));
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
        .insert(MainCharacter {
            dash_timer: Timer::from_seconds(1.0, TimerMode::Once),
            attract_timer: Timer::from_seconds(0.5, TimerMode::Once),
            explode_timer: Timer::from_seconds(0.8, TimerMode::Once),
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
        if sum >= level.point_threshold {
            if !prog.level_timer.finished() {
                prog.golden_apples += 1;
            }
            prog.current_level += 1;
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
                let progress_bar =
                    egui::ProgressBar::new(progress)
                        .show_percentage()
                        .text(format!(
                            "{:}% of level {}",
                            (progress * 100.0) as i32,
                            prog.current_level
                        ));
                ui.add(progress_bar);
            } else {
                if !prog.level_timer.finished() {
                    ui.label(format!(
                        "You won a golden apple! You now have {}",
                        prog.golden_apples
                    ));
                } else {
                    ui.label("Not fast enough!");
                }
            }
        });
    if !prog.end_timer.finished() && prog.level_complete {
        if [5, 10, 15].contains(&prog.golden_apples) {
            egui::Window::new("UpgProgress")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::splat(0.0))
                .show(egui_context.ctx_mut(), |ui| {
                    let text = match prog.golden_apples {
                        5 => "You have unlocked Dash! [Space]",
                        10 => "You have unlocked Magnet! [J]",
                        15 => "You have unlocked Dynamite! [K]",
                        _ => "Level passed",
                    };
                    ui.label(text);
                });
        }
    }
    egui::Window::new("TimeProgress")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::Vec2::splat(0.0))
        .show(egui_context.ctx_mut(), |ui| {
            ui.visuals_mut().selection.bg_fill = color;

            let text = if prog.level_timer.finished() {
                "Not fast enough for a golden apple."
            } else {
                "Time left"
            };
            let progress_bar = egui::ProgressBar::new(1.0 - prog.level_timer.percent()).text(text);
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

fn hit_fx(
    my_assets: Res<GameAssets>,
    audio: Res<Audio>,
    mut impact_events: EventReader<ContactForceEvent>,
) {
    for ev in impact_events.iter() {
        if ev.max_force_magnitude > 1.0 {
            audio.play_with_settings(
                my_assets.hit.clone(),
                PlaybackSettings {
                    repeat: false,
                    volume: 0.3,
                    ..default()
                },
            );
        }
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

#[derive(Clone, Debug)]
struct Rect {
    pos: Vec2,
    size: Vec2,
}

#[derive(Clone)]
struct Layout {
    rects: Vec<Rect>,
}

impl Layout {
    fn gen_quarter(mag: u32, rng: &mut ThreadRng, walls_prob: f32) -> Layout {
        let mut layout = Layout { rects: vec![] };
        for _ in 0..mag {
            // generate random box size
            let size = if rng.gen_bool(1.0 - walls_prob as f64) {
                Vec2::new(
                    rng.gen_range(2..10) as f32 * 5.0,
                    rng.gen_range(2..10) as f32 * 5.0,
                )
            } else {
                let long = rng.gen_range(15..25) as f32 * 5.0;
                let short = rng.gen_range(3..10) as f32 * 5.0;
                if rng.gen_bool(0.5) {
                    Vec2::new(long, short)
                } else {
                    Vec2::new(short, long)
                }
            };

            // select a direction (along x or y)
            let (dir, perp) = if rng.gen_bool(0.5) {
                (Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0))
            } else {
                (Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0))
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
                    let stepped = (max / 5.0) as i32;
                    rng.gen_range(0..stepped) as f32 * 5.0
                } else {
                    0.0
                };

            // move the box along other dir to not overlap the others
            let rect = Rect {
                pos: off * dir,
                size,
            };
            let casted = layout.cast(&(dir * off), &perp, &rect);

            layout.rects.push(casted);

            // mirror along x and y
        }
        layout.drag(0, &Vec2::new(1.0, 0.0), 40.0);
        layout.drag(0, &Vec2::new(0.0, 1.0), 40.0);
        for _ in 0..5 {
            let dir = if rng.gen_bool(0.5) {
                Vec2::new(1.0, 0.0)
            } else {
                Vec2::new(0.0, 1.0)
            };
            let dist = rng.gen_range(2..6) as f32 * 5.0;
            layout.drag(rng.gen_range(0..layout.rects.len()), &dir, dist);
        }
        layout.clip_oob(&Vec2::new(350.0, 290.0))
    }

    fn gen(mag: u32, rng: &mut ThreadRng, walls_prob: f32) -> Layout {
        let mut layout = Layout::gen_quarter(mag, rng, walls_prob);
        let stamp = layout.clone();
        layout = layout.merge(&stamp.mirror_x());
        layout = layout.merge(&stamp.mirror_y());
        layout = layout.merge(&stamp.mirror_y().mirror_x());
        layout.character_hole(32.0)
    }

    fn drag(&mut self, rect_i: usize, dir: &Vec2, distance: f32) {
        let mut dragging = vec![rect_i];
        for _ in 0..distance as i32 {
            let movevec = dir.clone();
            for _ in 0..100 {
                let mut pushed = vec![];
                for drag in dragging.iter() {
                    let mut moved = self.rects[*drag].clone();
                    moved.pos += movevec;
                    let mut inters: Vec<usize> = self
                        .rects
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| Layout::intersects(r, &moved))
                        .map(|(j, _)| j)
                        .collect();
                    pushed.append(&mut inters);
                }
                let mut finished = true;
                for p in pushed.iter() {
                    if !dragging.contains(p) {
                        dragging.push(*p);
                        finished = false;
                    }
                }
                if finished {
                    break;
                }
            }
            for drag in dragging.iter() {
                self.rects[*drag].pos += movevec;
            }
        }
    }

    fn clip_oob(&self, oob: &Vec2) -> Layout {
        Layout {
            rects: self
                .rects
                .iter()
                .filter(|r| {
                    !Layout::intersects(
                        r,
                        &Rect {
                            pos: Vec2::new(0.0, oob.y),
                            size: *oob,
                        },
                    ) && !Layout::intersects(
                        r,
                        &Rect {
                            pos: Vec2::new(oob.x, 0.0),
                            size: *oob,
                        },
                    )
                })
                .cloned()
                .collect(),
        }
    }

    fn character_hole(&self, rad: f32) -> Layout {
        Layout {
            rects: self
                .rects
                .iter()
                .filter(|r| {
                    r.pos.length() > rad
                        && (r.pos + r.size).length() > rad
                        && (r.pos + r.size * Vec2::new(0.0, 1.0)).length() > rad
                        && (r.pos + r.size * Vec2::new(1.0, 0.0)).length() > rad
                })
                .cloned()
                .collect(),
        }
    }

    fn merge(&self, oth: &Layout) -> Layout {
        let mut rects = self.rects.clone();
        let mut othrects = oth.rects.clone();
        rects.append(&mut othrects);
        Layout { rects }
    }

    fn mirror_x(&self) -> Layout {
        Layout {
            rects: self
                .rects
                .iter()
                .map(|r| {
                    let topleft = (r.pos + r.size) * Vec2::new(-1.0, 1.0);
                    let bottomleft = topleft - r.size * Vec2::new(0.0, 1.0);
                    Rect {
                        pos: bottomleft,
                        size: r.size,
                    }
                })
                .collect(),
        }
    }

    fn mirror_y(&self) -> Layout {
        Layout {
            rects: self
                .rects
                .iter()
                .map(|r| {
                    let bottomright = (r.pos + r.size) * Vec2::new(1.0, -1.0);
                    let bottomleft = bottomright - r.size * Vec2::new(1.0, 0.0);
                    Rect {
                        pos: bottomleft,
                        size: r.size,
                    }
                })
                .collect(),
        }
    }

    fn intersects(r: &Rect, rect: &Rect) -> bool {
        r.pos.x < rect.pos.x + rect.size.x
            && rect.pos.x < r.pos.x + r.size.x
            && r.pos.y < rect.pos.y + rect.size.y
            && rect.pos.y < r.pos.y + r.size.y
    }

    fn cast(&self, start: &Vec2, dir: &Vec2, rect: &Rect) -> Rect {
        let mut min = Vec2::new(0.0, 0.0);
        for i in 0..100 {
            for r in self.rects.iter() {
                let sample = r.pos * dir.clone();
                if (sample * dir.clone()).length() > min.length() {
                    min = sample;
                    break;
                }
                let sample = (r.pos + r.size) * dir.clone();
                if (sample * dir.clone()).length() > min.length() {
                    min = sample;
                    break;
                }
            }
            let casted = Rect {
                pos: min + start.clone(),
                size: rect.size.clone(),
            };
            if !self.rects.iter().any(|r| Layout::intersects(r, &casted)) {
                return casted;
            }
        }
        let mut pos = start.clone() + (dir.clone() * (self.rects.len() as f32 * 50.0));
        return Rect {
            pos,
            size: rect.size.clone(),
        };
    }
}

impl Level {
    fn gen(num: u32) -> Level {
        let mut rng = rand::thread_rng();
        let hue: f32 = rng.gen_range(0.0..360.0);
        let color = Color::hsl(hue, 1.0, 0.5);
        let target_color = Color::hsl((hue + 137.0).clamp(0.0, 360.0), 1.0, 0.5);
        let wall_color = Color::hsl((hue + 137.0 * 2.0).clamp(0.0, 360.0), 0.5, 0.4);
        let back_color = Color::hsl(0.0, 0.0, 0.1);
        let movable = Block {
            color,
            base_color: color * 0.7,
            color_target: target_color,
            ..default()
        };
        let wall = Block {
            color: wall_color,
            base_color: wall_color,
            color_target: target_color,
            density: 0.0,
            ..default()
        };
        let (mag, walls_prob) = match num {
            1..=2 => (5, 0.0),
            3..=6 => (10, 0.03),
            7..=15 => (20, 0.06),
            n => ((n * 2).max(40), 0.1),
        };
        let layout = Layout::gen(mag, &mut rng, walls_prob);
        let blocks: Vec<Block> = layout
            .rects
            .iter()
            .map(|r| {
                if r.size.length() >= 15.0 * 5.0 {
                    wall.mov(r.pos + r.size * 0.5).siz(r.size)
                } else {
                    movable.mov(r.pos + r.size * 0.5).siz(r.size)
                }
            })
            .collect();
        let blocknum = blocks.iter().filter(|b| b.density > 0.0).count();
        let wallnum = blocks.iter().filter(|b| b.density == 0.0).count();
        Level {
            id: num,
            back_color,
            blocks: blocks.clone(),
            point_threshold: blocknum as f32 * 0.76,
            spawnpoint: Vec3::new(0.0, 0.0, 20.0),
            duration: Duration::from_secs((blocknum + wallnum * 5) as u64),
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

#[derive(Component, Debug)]
struct MainCharacter {
    dash_timer: Timer,
    attract_timer: Timer,
    explode_timer: Timer,
}

#[derive(Component)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut TextureAtlasSprite)>,
    main_query: Query<(&Velocity, &MainCharacter)>,
) {
    if let Ok((vel, main)) = main_query.get_single() {
        for (mut timer, mut sprite) in &mut query {
            if !main.attract_timer.finished() && main.attract_timer.percent() < 0.25 {
                sprite.index = 8;
            } else if !main.explode_timer.finished() && main.explode_timer.percent() < 0.25 {
                sprite.index = 12;
            } else if !main.dash_timer.finished() && main.dash_timer.percent() < 0.25 {
                sprite.index = 4;
            } else {
                let speed = vel.linvel.length().min(1.0);
                timer.0.tick(time.delta() * speed as u32);
                if timer.0.finished() {
                    sprite.index = (sprite.index + 1) % 4;
                }
            }
        }
    }
}

fn audio_volumes(
    main_query: Query<(&Velocity, &MainCharacter)>,
    sinks: Res<Soundtrack>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    if let Ok((vel, _)) = main_query.get_single() {
        let speed = (vel.linvel.length() * 0.001).min(1.0);
        let amt = speed;

        if let Some(sink) = audio_sinks.get(&sinks.hi) {
            sink.set_volume(amt * 0.2);
        }
        if let Some(sink) = audio_sinks.get(&sinks.lo) {
            sink.set_volume((1.0 - amt) * 0.2);
        }
    }
}

const EXPLODE_COST: i32 = 15;
const EXPLODE_POWER: f32 = 100000.0;

fn explode(
    mut main_char: Query<(&mut MainCharacter, &Transform), (Without<Block>, With<MainCharacter>)>,
    mut block_query: Query<
        (&Block, &Transform, &mut Velocity, &ColliderMassProperties),
        (Without<MainCharacter>, With<Block>),
    >,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    prog: Res<Progress>,
    mut chroma: ResMut<ChromaticAberration>,
) {
    if let Ok((mut main, mtr)) = main_char.get_single_mut() {
        main.explode_timer.tick(time.delta());
        if main.explode_timer.finished()
            && keyboard_input.pressed(KeyCode::J)
            && prog.golden_apples >= EXPLODE_COST
        {
            main.explode_timer.reset();
            let mut affected = Vec2::new(0.0, 0.0);
            for (_, btr, mut v, coll) in block_query.iter_mut() {
                match coll {
                    ColliderMassProperties::Density(d) => {
                        if d == &0.0 {
                            continue;
                        }
                    }
                    _ => (),
                }
                let delta = mtr.translation - btr.translation;
                let delta = Vec2::new(delta.x, delta.y);
                let mag = delta.length();
                if mag > 20.0 && mag < 100.0 {
                    let pow = -delta
                        * (1.0 / (mag * mag))
                        * (time.delta_seconds() * 60.0)
                        * EXPLODE_POWER;
                    v.linvel += pow;
                    affected += pow;
                }
            }
            let amt = (affected / 500.0).length().min(0.5);
            chroma.magnitude_r += 0.02 * amt;
            chroma.magnitude_g += 0.02 * amt;
            chroma.magnitude_b += 0.02 * amt;
            chroma.dir_r += Vec2::new(1.0, 0.0);
            chroma.dir_g += Vec2::new(1.0, 1.0).normalize();
            chroma.dir_b += Vec2::new(-1.0, 1.0).normalize();
        }
    }
}

const ATTRACT_COST: i32 = 10;
const ATTRACT_POWER: f32 = 1000.0;

fn attract(
    mut main_char: Query<(&mut MainCharacter, &Transform), (Without<Block>, With<MainCharacter>)>,
    mut block_query: Query<
        (&Block, &Transform, &mut Velocity, &ColliderMassProperties),
        (Without<MainCharacter>, With<Block>),
    >,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    prog: Res<Progress>,
    mut chroma: ResMut<ChromaticAberration>,
) {
    if let Ok((mut main, mtr)) = main_char.get_single_mut() {
        main.attract_timer.tick(time.delta());
        if keyboard_input.pressed(KeyCode::K) && prog.golden_apples >= ATTRACT_COST {
            main.attract_timer.reset();
            let mut affected = Vec2::new(0.0, 0.0);
            for (_, btr, mut v, coll) in block_query.iter_mut() {
                match coll {
                    ColliderMassProperties::Density(d) => {
                        if d == &0.0 {
                            continue;
                        }
                    }
                    _ => (),
                }
                let delta = mtr.translation - btr.translation;
                let delta = Vec2::new(delta.x, delta.y);
                let mag = delta.length();
                if mag > 20.0 && mag < 100.0 {
                    let pow =
                        delta * (1.0 / (mag * mag)) * (time.delta_seconds() * 60.0) * ATTRACT_POWER;
                    v.linvel += pow;
                    affected += pow;
                }
            }
            let amt = (affected / 1000.0).length().min(0.1);
            chroma.magnitude_r += 0.02 * amt;
            chroma.magnitude_g += 0.02 * amt;
            chroma.magnitude_b += 0.02 * amt;
            chroma.dir_r += Vec2::new(1.0, 0.0);
            chroma.dir_g += Vec2::new(1.0, 1.0).normalize();
            chroma.dir_b += Vec2::new(-1.0, 1.0).normalize();
        }
    }
}

const ACCELERATION: f32 = 50.0;
const DASH_COST: i32 = 5;
const DASH_BOOST: f32 = 2.5;

fn movement(
    mut main_char: Query<(&mut MainCharacter, &Transform, &mut Velocity)>,
    mut touch_input: EventReader<TouchInput>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mouse_pos: Res<MousePos>,
    time: Res<Time>,
    prog: Res<Progress>,
    mut chroma: ResMut<ChromaticAberration>,
) {
    if let Ok((mut main, tr, mut vel)) = main_char.get_single_mut() {
        let pos = Vec2::new(tr.translation.x, tr.translation.y);
        let mut acc = ACCELERATION;
        main.dash_timer.tick(time.delta());
        if main.dash_timer.finished() {
            if keyboard_input.pressed(KeyCode::Space) && prog.golden_apples >= DASH_COST {
                main.dash_timer.reset();
                acc *= DASH_BOOST * 2.0;
                chroma.magnitude_r += 0.002;
                chroma.magnitude_g += 0.0002;
                chroma.magnitude_b += 0.0002;
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

        if main.dash_timer.percent() < 0.25 {
            chroma.magnitude_r += 0.002;
            chroma.magnitude_g += 0.0002;
            chroma.magnitude_b += 0.0002;
        }
        let len = vel.linvel.length();
        let mut normcut = vel.linvel;
        if len > 1.0 {
            normcut = vel.linvel.normalize_or_zero();
        }
        chroma.dir_r = -normcut;
        chroma.dir_g = normcut.perp();
        chroma.dir_b = -normcut.perp();
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
