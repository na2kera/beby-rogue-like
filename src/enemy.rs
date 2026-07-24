use bevy::prelude::*;
use rand::RngExt;

use crate::assets::SpriteAssets;
use crate::player::{PLAYER_SIZE, Player};
use crate::wave::{WaveInfo, WavePhase};
use crate::{ARENA_HEIGHT, ARENA_WIDTH, GameState, Health};

/// 壁からどれだけ内側に出現するか
const SPAWN_MARGIN: f32 = 50.0;

/// 敵の頭上に表示するHPバーの高さ
const HEALTH_BAR_HEIGHT: f32 = 6.0;

/// 敵スプライトの上端からHPバーまでの間隔
const HEALTH_BAR_GAP: f32 = 8.0;

/// 敵の種類
#[derive(Clone, Copy)]
pub enum EnemyKind {
    /// 標準（バランス型）
    Normal,
    /// 高速・低HP
    Fast,
    /// 低速・高HP
    Tank,
}

/// 敵1種類分の性能
pub struct EnemyStats {
    pub size: f32,
    pub speed: f32,
    pub max_hp: f32,
    pub contact_damage: f32,
    /// 倒したときに得られるスコア
    pub score: u32,
}

impl EnemyKind {
    pub fn stats(self) -> EnemyStats {
        match self {
            EnemyKind::Normal => EnemyStats {
                size: 56.0,
                speed: 150.0,
                max_hp: 30.0,
                contact_damage: 10.0,
                score: 10,
            },
            EnemyKind::Fast => EnemyStats {
                size: 44.0,
                speed: 240.0,
                max_hp: 15.0,
                contact_damage: 8.0,
                score: 15,
            },
            EnemyKind::Tank => EnemyStats {
                size: 84.0,
                speed: 90.0,
                max_hp: 90.0,
                contact_damage: 20.0,
                score: 30,
            },
        }
    }

    /// この種類の敵のスプライト画像を返す
    fn image(self, sprites: &SpriteAssets) -> Handle<Image> {
        match self {
            EnemyKind::Normal => sprites.enemy_normal.clone(),
            EnemyKind::Fast => sprites.enemy_fast.clone(),
            EnemyKind::Tank => sprites.enemy_tank.clone(),
        }
    }
}

/// 敵エンティティに付くコンポーネント。当たり判定や移動に使う性能を持つ
#[derive(Component)]
pub struct Enemy {
    pub size: f32,
    pub speed: f32,
    pub contact_damage: f32,
    /// 倒したときに得られるスコア
    pub score: u32,
}

/// 最終ボスのマーカー（Enemy と併用する）
#[derive(Component)]
pub struct Boss;

/// HPバーの残量部分（前景）に付くコンポーネント。
/// 敵本体の子エンティティなので、敵と一緒に消える
#[derive(Component)]
struct HealthBarFill {
    /// HPを参照する敵本体のエンティティ
    owner: Entity,
    /// HP満タン時のバーの幅
    full_width: f32,
}

/// 敵が倒されたことを他のシステムに知らせるメッセージ（ドロップ処理とスコア加算が購読する）
#[derive(Message)]
pub struct EnemyDied {
    pub position: Vec2,
    /// 倒した敵のスコア
    pub score: u32,
}

/// 敵の出現タイミングを管理するリソース（ワールドに1つだけのグローバルデータ）
#[derive(Resource)]
struct EnemySpawnTimer(Timer);

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnemyDied>()
            .insert_resource(EnemySpawnTimer(Timer::from_seconds(
                1.0,
                TimerMode::Repeating,
            )))
            .add_systems(
                Update,
                (spawn_enemies, chase_player, hit_player, despawn_dead_enemies)
                    .chain()
                    .run_if(in_state(WavePhase::Fighting)),
            )
            // HPバーはボス（OnEnter で出現）にも付けたいので、戦闘中に限らず Playing 全体で回す
            .add_systems(
                Update,
                (attach_health_bars, update_health_bars)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), despawn_all_enemies);
    }
}

/// ウェーブ番号に応じて敵の種類を抽選する。
/// Wave 3 から高速型、Wave 5 からタンク型が混ざり始める
fn pick_enemy_kind(wave_number: u32, roll: f32) -> EnemyKind {
    match wave_number {
        1..=2 => EnemyKind::Normal,
        3..=4 => {
            if roll < 0.25 {
                EnemyKind::Fast
            } else {
                EnemyKind::Normal
            }
        }
        _ => {
            if roll < 0.20 {
                EnemyKind::Tank
            } else if roll < 0.45 {
                EnemyKind::Fast
            } else {
                EnemyKind::Normal
            }
        }
    }
}

/// 一定間隔でアリーナの外周付近にランダムに敵を出現させる。
/// 出現間隔・HP・攻撃力・移動速度はウェーブが進むほど厳しくなる
fn spawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    wave: Res<WaveInfo>,
    sprites: Res<SpriteAssets>,
    mut timer: ResMut<EnemySpawnTimer>,
    bosses: Query<(), With<Boss>>,
) {
    // ボスウェーブはボス討伐後の増援を止める。
    // 残った敵を全滅させればウェーブクリアになる
    if wave.is_boss_wave() && wave.boss_spawned && bosses.is_empty() {
        return;
    }

    timer
        .0
        .set_duration(std::time::Duration::from_secs_f32(wave.spawn_interval_secs()));
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

    let kind = pick_enemy_kind(wave.number, rng.random());
    let stats = kind.stats();

    commands.spawn((
        Enemy {
            size: stats.size,
            speed: stats.speed * wave.enemy_speed_multiplier(),
            contact_damage: stats.contact_damage * wave.enemy_damage_multiplier(),
            score: stats.score,
        },
        Health::new(stats.max_hp * wave.enemy_hp_multiplier()),
        Sprite {
            image: kind.image(&sprites),
            custom_size: Some(Vec2::splat(stats.size)),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 0.5),
    ));
}

/// すべての敵がプレイヤーに向かって移動する
fn chase_player(
    time: Res<Time>,
    player: Single<&Transform, With<Player>>,
    mut enemies: Query<(&mut Transform, &Enemy), Without<Player>>,
) {
    let player_position = player.translation.truncate();

    for (mut transform, enemy) in &mut enemies {
        let direction = (player_position - transform.translation.truncate()).normalize_or_zero();
        let movement = direction * enemy.speed * time.delta_secs();
        transform.translation += movement.extend(0.0);
    }
}

/// 敵と接触したらプレイヤーにダメージを与える（無敵時間中は無効）
fn hit_player(
    mut player: Single<(&Transform, &mut Player, &mut Health)>,
    enemies: Query<(&Transform, &Enemy)>,
) {
    let (player_transform, player, health) = &mut *player;

    if !player.invincible_timer.is_finished() {
        return;
    }

    let player_position = player_transform.translation.truncate();

    // 中心間の距離で接触を判定する（円同士の当たり判定の近似）
    let touching_damage = enemies.iter().find_map(|(enemy_transform, enemy)| {
        let hit_distance = (PLAYER_SIZE + enemy.size) / 2.0;
        let distance = player_position.distance(enemy_transform.translation.truncate());
        (distance < hit_distance).then_some(enemy.contact_damage)
    });

    if let Some(damage) = touching_damage {
        health.current -= damage;
        player.invincible_timer.reset();
    }
}

/// 出現した敵（ボス含む）の頭上にHPバーを子エンティティとして付ける
fn attach_health_bars(mut commands: Commands, enemies: Query<(Entity, &Enemy), Added<Enemy>>) {
    for (entity, enemy) in &enemies {
        let width = enemy.size;
        let offset_y = enemy.size / 2.0 + HEALTH_BAR_GAP;

        commands.entity(entity).with_children(|parent| {
            // 背景（減った分が見える暗い帯）
            parent.spawn((
                Sprite::from_color(
                    Color::srgb(0.15, 0.15, 0.15),
                    Vec2::new(width, HEALTH_BAR_HEIGHT),
                ),
                Transform::from_xyz(0.0, offset_y, 0.1),
            ));
            // 前景（残りHP。長さと色を update_health_bars が更新する）
            parent.spawn((
                HealthBarFill {
                    owner: entity,
                    full_width: width,
                },
                Sprite::from_color(
                    Color::srgb(0.25, 0.85, 0.30),
                    Vec2::new(width, HEALTH_BAR_HEIGHT),
                ),
                Transform::from_xyz(0.0, offset_y, 0.2),
            ));
        });
    }
}

/// HPバーの長さと色を残りHPに合わせて更新する。
/// バーは左端を基準に縮むよう、幅に応じて位置もずらす
fn update_health_bars(
    healths: Query<&Health, With<Enemy>>,
    mut bars: Query<(&HealthBarFill, &mut Sprite, &mut Transform)>,
) {
    for (bar, mut sprite, mut transform) in &mut bars {
        let Ok(health) = healths.get(bar.owner) else {
            continue;
        };
        let ratio = (health.current / health.max).clamp(0.0, 1.0);

        sprite.custom_size = Some(Vec2::new(bar.full_width * ratio, HEALTH_BAR_HEIGHT));
        transform.translation.x = -bar.full_width * (1.0 - ratio) / 2.0;
        sprite.color = if ratio > 0.5 {
            Color::srgb(0.25, 0.85, 0.30) // 緑
        } else if ratio > 0.25 {
            Color::srgb(0.95, 0.80, 0.20) // 黄
        } else {
            Color::srgb(0.90, 0.25, 0.20) // 赤
        };
    }
}

/// ラン終了時に残っている敵（ボス含む）をすべて消す
fn despawn_all_enemies(mut commands: Commands, enemies: Query<Entity, With<Enemy>>) {
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
}

/// HPが0以下になった敵を消し、死亡メッセージを送る
///（スコア加算とドロップ処理がこのメッセージを購読する）
fn despawn_dead_enemies(
    mut commands: Commands,
    mut died: MessageWriter<EnemyDied>,
    enemies: Query<(Entity, &Transform, &Health, &Enemy)>,
) {
    for (entity, transform, health, enemy) in &enemies {
        if health.current <= 0.0 {
            died.write(EnemyDied {
                position: transform.translation.truncate(),
                score: enemy.score,
            });
            commands.entity(entity).despawn();
        }
    }
}
