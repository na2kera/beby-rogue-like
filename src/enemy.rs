use bevy::prelude::*;
use rand::RngExt;

use crate::player::{PLAYER_SIZE, Player};
use crate::{ARENA_HEIGHT, ARENA_WIDTH, Health};

pub const ENEMY_SIZE: f32 = 28.0;
const ENEMY_SPEED: f32 = 150.0;
const ENEMY_MAX_HP: f32 = 30.0;

/// 接触1回あたりのダメージ
const CONTACT_DAMAGE: f32 = 10.0;

/// 敵の出現間隔（秒）
const SPAWN_INTERVAL_SECS: f32 = 1.0;

/// 壁からどれだけ内側に出現するか
const SPAWN_MARGIN: f32 = 50.0;

/// 敵であることを示すマーカーコンポーネント
#[derive(Component)]
pub struct Enemy;

/// 敵の出現タイミングを管理するリソース（ワールドに1つだけのグローバルデータ）
#[derive(Resource)]
struct EnemySpawnTimer(Timer);

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemySpawnTimer(Timer::from_seconds(
            SPAWN_INTERVAL_SECS,
            TimerMode::Repeating,
        )))
        .add_systems(Update, (spawn_enemies, chase_player, hit_player).chain());
    }
}

/// 一定間隔でアリーナの外周付近にランダムに敵を出現させる
fn spawn_enemies(mut commands: Commands, time: Res<Time>, mut timer: ResMut<EnemySpawnTimer>) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let mut rng = rand::rng();
    let half_w = ARENA_WIDTH / 2.0 - SPAWN_MARGIN;
    let half_h = ARENA_HEIGHT / 2.0 - SPAWN_MARGIN;

    // 上下左右の4辺からランダムに1辺を選び、その辺に沿ったランダムな位置に出す
    let position = match rng.random_range(0..4) {
        0 => Vec2::new(rng.random_range(-half_w..half_w), half_h), // 上
        1 => Vec2::new(rng.random_range(-half_w..half_w), -half_h), // 下
        2 => Vec2::new(-half_w, rng.random_range(-half_h..half_h)), // 左
        _ => Vec2::new(half_w, rng.random_range(-half_h..half_h)), // 右
    };

    commands.spawn((
        Enemy,
        Health::new(ENEMY_MAX_HP),
        Sprite::from_color(Color::srgb(0.9, 0.25, 0.25), Vec2::splat(ENEMY_SIZE)),
        Transform::from_xyz(position.x, position.y, 0.5),
    ));
}

/// すべての敵がプレイヤーに向かって移動する
fn chase_player(
    time: Res<Time>,
    player: Single<&Transform, With<Player>>,
    mut enemies: Query<&mut Transform, (With<Enemy>, Without<Player>)>,
) {
    let player_position = player.translation.truncate();

    for mut transform in &mut enemies {
        let direction = (player_position - transform.translation.truncate()).normalize_or_zero();
        let movement = direction * ENEMY_SPEED * time.delta_secs();
        transform.translation += movement.extend(0.0);
    }
}

/// 敵と接触したらプレイヤーにダメージを与える（無敵時間中は無効）
fn hit_player(
    mut player: Single<(&Transform, &mut Player, &mut Health)>,
    enemies: Query<&Transform, With<Enemy>>,
) {
    let (player_transform, player, health) = &mut *player;

    if !player.invincible_timer.is_finished() {
        return;
    }

    // 中心間の距離で接触を判定する（円同士の当たり判定の近似）
    let hit_distance = (PLAYER_SIZE + ENEMY_SIZE) / 2.0;
    let player_position = player_transform.translation.truncate();

    let touching = enemies.iter().any(|enemy_transform| {
        player_position.distance(enemy_transform.translation.truncate()) < hit_distance
    });

    if touching {
        health.current -= CONTACT_DAMAGE;
        player.invincible_timer.reset();
    }
}
