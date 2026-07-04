use bevy::prelude::*;

use crate::{ARENA_HEIGHT, ARENA_WIDTH, Health, WINDOW_HEIGHT, WINDOW_WIDTH};

// プレイヤーの移動速度（ピクセル/秒）とサイズ
pub const PLAYER_SPEED: f32 = 300.0;
pub const PLAYER_SIZE: f32 = 32.0;
const PLAYER_MAX_HP: f32 = 100.0;

/// 被弾後の無敵時間（秒）
const HIT_INVINCIBLE_SECS: f32 = 0.5;

/// プレイヤーのコンポーネント。被弾後の無敵時間タイマーを持つ
#[derive(Component)]
pub struct Player {
    pub invincible_timer: Timer,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player).add_systems(
            Update,
            (move_player, tick_invincibility, check_player_death, camera_follow).chain(),
        );
    }
}

fn spawn_player(mut commands: Commands) {
    // 無敵タイマーは「経過済み」の状態で持たせ、開始直後から被弾できるようにする
    let mut invincible_timer = Timer::from_seconds(HIT_INVINCIBLE_SECS, TimerMode::Once);
    invincible_timer.finish();

    commands.spawn((
        Player { invincible_timer },
        Health::new(PLAYER_MAX_HP),
        Sprite::from_color(Color::srgb(0.2, 0.5, 1.0), Vec2::splat(PLAYER_SIZE)),
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
}

/// WASD / 矢印キーの入力でプレイヤーを移動させる
fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: Single<&mut Transform, With<Player>>,
) {
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    // 斜め移動が速くならないように長さを1に揃える（入力なしなら零ベクトルのまま）
    let direction = direction.normalize_or_zero();

    let mut position =
        player.translation.truncate() + direction * PLAYER_SPEED * time.delta_secs();

    // アリーナの壁の内側にとどめる
    let bound = Vec2::new(ARENA_WIDTH, ARENA_HEIGHT) / 2.0 - PLAYER_SIZE / 2.0;
    position = position.clamp(-bound, bound);

    player.translation = position.extend(player.translation.z);
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

/// HPが0になったらプレイヤーを消す（ゲームオーバー画面はマイルストーン⑦で実装）
fn check_player_death(
    mut commands: Commands,
    player: Single<(Entity, &Health), With<Player>>,
) {
    let (entity, health) = *player;
    if health.current <= 0.0 {
        info!("GAME OVER");
        commands.entity(entity).despawn();
    }
}

/// カメラをプレイヤーに追従させる（ただしアリーナの外は映さない）
fn camera_follow(
    player: Single<&Transform, With<Player>>,
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    // カメラ中心が動ける範囲 = アリーナ半分 - 画面半分
    let bound = Vec2::new(
        (ARENA_WIDTH - WINDOW_WIDTH) / 2.0,
        (ARENA_HEIGHT - WINDOW_HEIGHT) / 2.0,
    )
    .max(Vec2::ZERO);

    let target = player.translation.truncate().clamp(-bound, bound);
    camera.translation = target.extend(camera.translation.z);
}
