mod assets;
mod enemy;
mod input;
mod lang;
mod pickup;
mod player;
mod screens;
mod ui;
mod wave;
mod weapon;

use bevy::prelude::*;
use bevy::sprite::SpriteImageMode;

use crate::assets::SpriteAssets;
use crate::weapon::WeaponType;

// ウィンドウサイズ
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 720.0;

// アリーナ（壁に囲まれた戦闘フィールド）のサイズ
pub const ARENA_WIDTH: f32 = 2400.0;
pub const ARENA_HEIGHT: f32 = 1600.0;

/// アリーナを囲む壁の厚み（壁タイル1枚分）
pub const WALL_THICKNESS: f32 = 64.0;

/// ゲーム全体の画面フロー
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameState {
    #[default]
    Title,
    Playing,
    Result,
}

/// 1ランの結果。ランが終わった瞬間に記録し、リザルト画面が表示に使う
#[derive(Resource)]
pub struct RunResult {
    pub victory: bool,
    pub wave_reached: u32,
    /// ラン終了時の所持武器（種類とレベル）
    pub weapons: Vec<(WeaponType, u8)>,
}

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
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Beby Rogue Like".into(),
                        resolution: (WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                // ドット絵がぼやけないよう、拡大縮小を最近傍補間にする
                .set(ImagePlugin::default_nearest()),
        )
        .init_state::<GameState>()
        .add_plugins((
            assets::SpriteAssetsPlugin,
            input::GameInputPlugin,
            lang::LangPlugin,
            player::PlayerPlugin,
            enemy::EnemyPlugin,
            weapon::WeaponPlugin,
            pickup::PickupPlugin,
            wave::WavePlugin,
            ui::UiPlugin,
            screens::ScreensPlugin,
        ))
        .add_systems(Startup, setup_arena)
        .run();
}

/// カメラとアリーナの床・壁を生成する（全画面で共通なので起動時に一度だけ）
fn setup_arena(mut commands: Commands, sprites: Res<SpriteAssets>) {
    commands.spawn(Camera2d);

    // アリーナの床（64x64 のタイル画像を敷き詰める）
    commands.spawn((
        tiled_sprite(sprites.floor_tile.clone(), Vec2::new(ARENA_WIDTH, ARENA_HEIGHT)),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    // アリーナを囲む壁。上下の帯は角も覆うよう左右の厚み分だけ長くする
    let horizontal = Vec2::new(ARENA_WIDTH + WALL_THICKNESS * 2.0, WALL_THICKNESS);
    let vertical = Vec2::new(WALL_THICKNESS, ARENA_HEIGHT);
    let offset_x = (ARENA_WIDTH + WALL_THICKNESS) / 2.0;
    let offset_y = (ARENA_HEIGHT + WALL_THICKNESS) / 2.0;

    for (size, position) in [
        (horizontal, Vec2::new(0.0, offset_y)),  // 上
        (horizontal, Vec2::new(0.0, -offset_y)), // 下
        (vertical, Vec2::new(-offset_x, 0.0)),   // 左
        (vertical, Vec2::new(offset_x, 0.0)),    // 右
    ] {
        commands.spawn((
            tiled_sprite(sprites.wall_tile.clone(), size),
            Transform::from_xyz(position.x, position.y, -0.5),
        ));
    }
}

/// 指定サイズいっぱいにタイル画像を敷き詰めるスプライトを作る
fn tiled_sprite(image: Handle<Image>, size: Vec2) -> Sprite {
    Sprite {
        image,
        custom_size: Some(size),
        image_mode: SpriteImageMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 1.0,
        },
        ..default()
    }
}
