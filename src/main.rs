use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use bevy_rapier2d::prelude::*;
use bevy_vfx_bag::{
    image::{chromatic_aberration::ChromaticAberrationPlugin, pixelate::PixelatePlugin},
    BevyVfxBagPlugin, PostProcessingInput,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                fit_canvas_to_parent: true,
                ..default()
            },
            ..default()
        }))
        .add_loading_state(
            LoadingState::new(AssetStates::AssetLoading)
                .continue_to_state(AssetStates::Next)
                .with_collection::<GameAssets>(),
        )
        .add_state(AssetStates::AssetLoading)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup_graphics)
        .add_system_set(SystemSet::on_enter(AssetStates::Next).with_system(setup_physics))
        .add_system_set(SystemSet::on_update(AssetStates::Next).with_system(animate_sprite_system))
        .add_system(bonk)
        .add_plugin(EguiPlugin)
        .add_system_set(SystemSet::on_update(AssetStates::Next).with_system(ui_example))
        .add_system_set(SystemSet::on_update(AssetStates::Next).with_system(movement))
        .insert_resource(Msaa { samples: 1 })
        .add_plugin(BevyVfxBagPlugin)
        .add_plugin(ChromaticAberrationPlugin)
        .add_plugin(PixelatePlugin)
        .run();
}

#[derive(AssetCollection, Resource)]
struct GameAssets {
    #[asset(texture_atlas(tile_size_x = 64., tile_size_y = 64., columns = 4, rows = 2))]
    #[asset(path = "sheet.png")]
    lolle: Handle<TextureAtlas>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum AssetStates {
    AssetLoading,
    Next,
}

fn setup_graphics(mut commands: Commands) {
    commands
        .spawn(Camera2dBundle::default())
        .insert(PostProcessingInput);
}

fn setup_physics(
    mut commands: Commands,
    my_assets: Res<GameAssets>,
    mut egui_context: ResMut<EguiContext>,
    mut egui_settings: ResMut<EguiSettings>,
) {
    let ctx = egui_context.ctx_mut();
    let mut style: egui::Style = (*ctx.style()).clone();
    style.visuals.window_fill = egui::Color32::from_rgb(0, 0, 0);
    style.visuals.window_rounding = egui::Rounding::none();
    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.override_text_color = Some(egui::Color32::WHITE);
    style.spacing.item_spacing = egui::vec2(10.0, 20.0);
    ctx.set_style(style);

    egui_settings.scale_factor = 2.0;

    commands
        .spawn(Collider::cuboid(500.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -100.0, 0.0)));

    // draw single texture from sprite sheet starting at index 0
    commands
        .spawn(SpriteSheetBundle {
            sprite: TextureAtlasSprite::new(0),
            texture_atlas: my_assets.lolle.clone(),
            ..Default::default()
        })
        .insert(AnimationTimer(Timer::from_seconds(
            0.2,
            TimerMode::Repeating,
        )))
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball(50.0))
        .insert(Restitution::coefficient(0.7))
        .insert(Velocity {
            linvel: Vec2::new(1.0, 2.0),
            angvel: 0.2,
        })
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(Loller)
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 400.0, 0.0)));
}

fn bonk(mut collision_events: EventReader<CollisionEvent>, mut egui_context: ResMut<EguiContext>) {
    for collision_event in collision_events.iter() {
        match collision_event {
            CollisionEvent::Started(_, _, flags) => egui::Window::new("AAA")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::LEFT_CENTER, egui::Vec2::splat(0.0))
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label(format!("{:?}", collision_event));
                }),
            _ => None,
        };
    }
}

#[derive(Component)]
struct Loller;

#[derive(Component)]
struct AnimationTimer(Timer);

fn animate_sprite_system(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut TextureAtlasSprite)>,
) {
    for (mut timer, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            sprite.index = (sprite.index + 1) % 8;
        }
    }
}

fn movement(mut lol: Query<(&Loller, &mut Velocity)>, keyboard_input: Res<Input<KeyCode>>) {
    if let Ok((_, mut vel)) = lol.get_single_mut() {
        vel.linvel.y += keyboard_input.pressed(KeyCode::W) as i32 as f32 * 5.0;
        vel.linvel.x -= keyboard_input.pressed(KeyCode::A) as i32 as f32 * 2.0;
        vel.linvel.x += keyboard_input.pressed(KeyCode::D) as i32 as f32 * 2.0;
    }
}

fn ui_example(
    mut egui_context: ResMut<EguiContext>,
    lol: Query<(&Loller, &Transform)>,
    my_assets: Res<GameAssets>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    let atlas = texture_atlases
        .get(&my_assets.lolle)
        .expect("Failed to find our atlas");
    let pos = lol.get_single().unwrap().1.translation;
    if pos.y > 10.0 {
        egui::Window::new("Hello")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::splat(0.0))
            .show(egui_context.ctx_mut(), |ui| {
                ui.heading("Loolll");
                ui.label("ho tante cose da dire");
                ui.label("tipo");
                ui.label("lolololol olo lol");
            });
    } else {
        egui::Window::new("Hello2")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::Vec2::splat(0.0))
            .show(egui_context.ctx_mut(), |ui| {
                ui.label("lolololol olo lol");
            });
    }
    egui::Window::new("Money")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::splat(0.0))
        .show(egui_context.ctx_mut(), |ui| {
            ui.label("0/40");
        });
}
