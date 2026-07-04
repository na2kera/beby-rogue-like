mod enemy;
mod player;
mod ui;

use bevy::prelude::*;

// ウィンドウサイズ
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 720.0;

// アリーナ（壁に囲まれた戦闘フィールド）のサイズ
pub const ARENA_WIDTH: f32 = 2400.0;
pub const ARENA_HEIGHT: f32 = 1600.0;

/// HPを持つもの（プレイヤー・敵）共通のコンポーネント
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Beby Rogue Like".into(),
                resolution: (WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((player::PlayerPlugin, enemy::EnemyPlugin, ui::UiPlugin))
        .add_systems(Startup, setup_arena)
        .run();
}

/// カメラとアリーナの床を生成する
fn setup_arena(mut commands: Commands) {
    commands.spawn(Camera2d);

    // アリーナの床（暗い青灰色の大きな矩形）
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.15, 0.15, 0.2),
            Vec2::new(ARENA_WIDTH, ARENA_HEIGHT),
        ),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));
}
