use bevy::prelude::*;

use crate::assets::{PLAYER_WALK_FRAME_COUNT, SpriteAssets};
use crate::input::{GameInputSet, MoveInput};
use crate::wave::{WaveInfo, WavePhase};
use crate::weapon::Weapon;
use crate::{ARENA_HEIGHT, ARENA_WIDTH, GameState, Health, RunResult, WALL_THICKNESS};

// プレイヤーの移動速度（ピクセル/秒）とサイズ
pub const PLAYER_SPEED: f32 = 300.0;
pub const PLAYER_SIZE: f32 = 64.0;
const PLAYER_MAX_HP: f32 = 100.0;

/// 被弾後の無敵時間（秒）
const HIT_INVINCIBLE_SECS: f32 = 0.5;

/// 歩行アニメーションのコマ送り間隔（秒）
const WALK_FRAME_SECS: f32 = 0.12;

/// プレイヤーのコンポーネント。被弾後の無敵時間タイマーを持つ
#[derive(Component)]
pub struct Player {
    pub invincible_timer: Timer,
    /// このフレームに移動入力があったか（歩行アニメーションの切替に使う）
    moving: bool,
}

/// 歩行アニメーションのコマ送りタイマー
#[derive(Component)]
struct WalkAnimation(Timer);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_player)
            .add_systems(
                Update,
                (move_player, animate_walk)
                    .chain()
                    .after(GameInputSet)
                    .run_if(in_state(WavePhase::Fighting)),
            )
            .add_systems(
                Update,
                (tick_invincibility, check_player_death, camera_follow).chain(),
            )
            .add_systems(OnExit(GameState::Playing), despawn_player);
    }
}

fn spawn_player(mut commands: Commands, sprites: Res<SpriteAssets>) {
    // 無敵タイマーは「経過済み」の状態で持たせ、開始直後から被弾できるようにする
    let mut invincible_timer = Timer::from_seconds(HIT_INVINCIBLE_SECS, TimerMode::Once);
    invincible_timer.finish();

    commands.spawn((
        Player {
            invincible_timer,
            moving: false,
        },
        WalkAnimation(Timer::from_seconds(WALK_FRAME_SECS, TimerMode::Repeating)),
        Health::new(PLAYER_MAX_HP),
        Sprite {
            image: sprites.player.clone(),
            custom_size: Some(Vec2::splat(PLAYER_SIZE)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
}

/// キーボード / 仮想スティックの入力でプレイヤーを移動させる
fn move_player(
    move_input: Res<MoveInput>,
    time: Res<Time>,
    mut player: Single<(&mut Transform, &mut Player)>,
) {
    let (transform, state) = &mut *player;

    let direction = move_input.0;
    state.moving = direction != Vec2::ZERO;

    let mut position =
        transform.translation.truncate() + direction * PLAYER_SPEED * time.delta_secs();

    // アリーナの壁の内側にとどめる
    let bound = Vec2::new(ARENA_WIDTH, ARENA_HEIGHT) / 2.0 - PLAYER_SIZE / 2.0;
    position = position.clamp(-bound, bound);

    transform.translation = position.extend(transform.translation.z);
}

/// 移動中は歩行スプライトシートをコマ送りし、停止中は立ち絵に戻す
fn animate_walk(
    time: Res<Time>,
    sprites: Res<SpriteAssets>,
    mut player: Single<(&Player, &mut Sprite, &mut WalkAnimation)>,
) {
    let (state, sprite, animation) = &mut *player;

    if !state.moving {
        sprite.image = sprites.player.clone();
        sprite.texture_atlas = None;
        animation.0.reset();
        return;
    }

    sprite.image = sprites.player_walk.clone();
    // 歩き始めた瞬間にアトラスを設定し、以降はコマ番号だけ進める
    let atlas = sprite.texture_atlas.get_or_insert_with(|| TextureAtlas {
        layout: sprites.player_walk_layout.clone(),
        index: 0,
    });

    animation.0.tick(time.delta());
    if animation.0.just_finished() {
        atlas.index = (atlas.index + 1) % PLAYER_WALK_FRAME_COUNT;
    }
}

/// 無敵時間タイマーを進め、無敵中はスプライトを半透明にして視覚的に分かるようにする
fn tick_invincibility(time: Res<Time>, mut player: Single<(&mut Player, &mut Sprite)>) {
    let (player, sprite) = &mut *player;
    player.invincible_timer.tick(time.delta());

    let alpha = if player.invincible_timer.is_finished() {
        1.0
    } else {
        0.4
    };
    sprite.color.set_alpha(alpha);
}

/// HPが0になったらランの結果を記録してリザルト画面へ移行する
fn check_player_death(
    mut commands: Commands,
    player: Single<&Health, With<Player>>,
    wave: Res<WaveInfo>,
    weapons: Query<&Weapon>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if player.current <= 0.0 {
        commands.insert_resource(RunResult {
            victory: false,
            wave_reached: wave.number,
            weapons: weapons.iter().map(|w| (w.weapon_type, w.level)).collect(),
        });
        next_state.set(GameState::Result);
    }
}

/// ラン終了時にプレイヤーを消す
fn despawn_player(mut commands: Commands, players: Query<Entity, With<Player>>) {
    for entity in &players {
        commands.entity(entity).despawn();
    }
}

/// カメラをプレイヤーに追従させる（ただしアリーナの外は映さない）
#[allow(clippy::type_complexity)]
fn camera_follow(
    player: Single<&Transform, With<Player>>,
    mut camera: Single<(&mut Transform, &Projection), (With<Camera2d>, Without<Player>)>,
) {
    let (transform, projection) = &mut *camera;

    // 画面に実際に映っている範囲。アスペクト比によって横幅が変わるため、
    // 固定のウィンドウサイズではなく投影の area から毎フレーム取得する
    let Projection::Orthographic(ortho) = projection else {
        return;
    };
    let visible = ortho.area.size();

    // カメラ中心が動ける範囲 = (アリーナ + 両側の壁)の半分 - 画面半分。
    // 壁の分だけ広げることで、端に寄ったとき壁タイルが画面に映る
    let bound = ((Vec2::new(ARENA_WIDTH, ARENA_HEIGHT) + WALL_THICKNESS * 2.0 - visible) / 2.0)
        .max(Vec2::ZERO);

    let target = player.translation.truncate().clamp(-bound, bound);
    transform.translation = target.extend(transform.translation.z);
}
