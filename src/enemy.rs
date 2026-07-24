use std::f32::consts::TAU;

use bevy::prelude::*;
use rand::RngExt;

use crate::assets::SpriteAssets;
use crate::player::{PLAYER_SIZE, Player};
use crate::wave::{WaveInfo, WavePhase};
use crate::{ARENA_HEIGHT, ARENA_WIDTH, GameState, Health};

/// 壁からどれだけ内側に出現するか
const SPAWN_MARGIN: f32 = 50.0;

/// 敵弾の当たり判定と見た目のサイズ
const BULLET_SIZE: f32 = 14.0;

/// 敵弾の寿命（秒）。射程の代わり
const BULLET_TTL_SECS: f32 = 4.0;

/// ジグザグ移動の蛇行速度（ラジアン/秒）
const ZIGZAG_FREQUENCY: f32 = 6.0;

/// ジグザグ移動で進行方向から最大どれだけ振れるか（ラジアン）
const ZIGZAG_MAX_ANGLE: f32 = 0.9;

/// 射撃型がプレイヤーと保とうとする距離
const SHOOTER_PREFERRED_DISTANCE: f32 = 380.0;

/// 距離維持の許容幅。この範囲内なら前進も後退もせず横移動する
const KEEP_DISTANCE_BAND: f32 = 40.0;

/// 敵の種類
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyKind {
    /// 標準（バランス型）
    Normal,
    /// 高速・低HP。ジグザグに蛇行しながら迫る
    Fast,
    /// 低速・高HP
    Tank,
    /// 距離を保ちながら弾を撃ってくる射撃型
    Shooter,
}

/// 敵1種類分の性能
pub struct EnemyStats {
    pub size: f32,
    pub speed: f32,
    pub max_hp: f32,
    pub contact_damage: f32,
}

impl EnemyKind {
    pub fn stats(self) -> EnemyStats {
        match self {
            EnemyKind::Normal => EnemyStats {
                size: 56.0,
                speed: 150.0,
                max_hp: 30.0,
                contact_damage: 10.0,
            },
            EnemyKind::Fast => EnemyStats {
                size: 44.0,
                speed: 240.0,
                max_hp: 15.0,
                contact_damage: 8.0,
            },
            EnemyKind::Tank => EnemyStats {
                size: 84.0,
                speed: 90.0,
                max_hp: 90.0,
                contact_damage: 20.0,
            },
            EnemyKind::Shooter => EnemyStats {
                size: 48.0,
                speed: 130.0,
                max_hp: 20.0,
                contact_damage: 6.0,
            },
        }
    }

    /// この種類の敵のスプライト画像を返す
    fn image(self, sprites: &SpriteAssets) -> Handle<Image> {
        match self {
            EnemyKind::Normal => sprites.enemy_normal.clone(),
            EnemyKind::Fast => sprites.enemy_fast.clone(),
            EnemyKind::Tank => sprites.enemy_tank.clone(),
            // 射撃型は専用画像がないため、標準型の画像を紫に着色して区別する
            EnemyKind::Shooter => sprites.enemy_normal.clone(),
        }
    }

    /// スプライトの着色。専用画像を持たない種類の見分けに使う
    fn color(self) -> Color {
        match self {
            EnemyKind::Shooter => Color::srgb(0.75, 0.55, 1.0),
            _ => Color::WHITE,
        }
    }
}

/// 敵の移動パターン
#[derive(Component, Clone, Copy)]
pub enum MovePattern {
    /// プレイヤーへ直進する
    Chase,
    /// プレイヤーへ向かいつつ左右に蛇行する。
    /// phase は個体ごとの蛇行タイミングのずれ（全員同期して揺れると不自然なため）
    Zigzag { phase: f32 },
    /// プレイヤーと一定距離を保つ。近づかれたら下がり、離れたら詰める。
    /// strafe_dir は距離維持中に回り込む向き（+1 か -1）
    KeepDistance { preferred: f32, strafe_dir: f32 },
}

/// 弾を撃つ敵に付くコンポーネント（射撃型とボス）
#[derive(Component)]
pub struct RangedAttacker {
    pub timer: Timer,
    pub bullet_speed: f32,
    pub bullet_damage: f32,
    /// 1回の射撃で撃つ弾数。2発以上は spread の間隔で扇状に広がる
    pub bullet_count: u32,
    /// 弾同士の角度間隔（ラジアン）。TAU / bullet_count にすると全周弾になる
    pub spread: f32,
}

/// 敵が撃った弾。プレイヤーにだけ当たる
#[derive(Component)]
pub struct EnemyBullet {
    velocity: Vec2,
    damage: f32,
    ttl: Timer,
}

/// 敵エンティティに付くコンポーネント。当たり判定や移動に使う性能を持つ
#[derive(Component)]
pub struct Enemy {
    pub size: f32,
    pub speed: f32,
    pub contact_damage: f32,
}

/// 最終ボスのマーカー（Enemy と併用する）
#[derive(Component)]
pub struct Boss;

/// 敵が倒されたことを他のシステムに知らせるメッセージ（ドロップ処理が購読する）
#[derive(Message)]
pub struct EnemyDied {
    pub position: Vec2,
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
                (
                    spawn_enemies,
                    move_enemies,
                    fire_ranged_attacks,
                    move_enemy_bullets,
                    hit_player,
                    bullet_hit_player,
                    despawn_dead_enemies,
                )
                    .chain()
                    .run_if(in_state(WavePhase::Fighting)),
            )
            // ウェーブ間は敵が一掃されるので、飛んでいる弾も一緒に消す
            .add_systems(OnEnter(WavePhase::Intermission), despawn_enemy_bullets)
            .add_systems(
                OnExit(GameState::Playing),
                (despawn_all_enemies, despawn_enemy_bullets),
            );
    }
}

/// ウェーブ番号に応じて敵の種類を抽選する。
/// Wave 3 から高速型、Wave 5 からタンク型、Wave 6 から射撃型が混ざり始める
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
        5 => {
            if roll < 0.20 {
                EnemyKind::Tank
            } else if roll < 0.45 {
                EnemyKind::Fast
            } else {
                EnemyKind::Normal
            }
        }
        _ => {
            if roll < 0.15 {
                EnemyKind::Shooter
            } else if roll < 0.35 {
                EnemyKind::Tank
            } else if roll < 0.60 {
                EnemyKind::Fast
            } else {
                EnemyKind::Normal
            }
        }
    }
}

/// 種類ごとの移動パターンを決める。個体差はここで乱数から与える
fn pick_move_pattern(kind: EnemyKind, rng: &mut impl RngExt) -> MovePattern {
    match kind {
        EnemyKind::Fast => MovePattern::Zigzag {
            phase: rng.random_range(0.0..TAU),
        },
        EnemyKind::Shooter => MovePattern::KeepDistance {
            preferred: SHOOTER_PREFERRED_DISTANCE,
            strafe_dir: if rng.random_bool(0.5) { 1.0 } else { -1.0 },
        },
        _ => MovePattern::Chase,
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

    let mut enemy = commands.spawn((
        Enemy {
            size: stats.size,
            speed: stats.speed * wave.enemy_speed_multiplier(),
            contact_damage: stats.contact_damage * wave.enemy_damage_multiplier(),
        },
        pick_move_pattern(kind, &mut rng),
        Health::new(stats.max_hp * wave.enemy_hp_multiplier()),
        Sprite {
            image: kind.image(&sprites),
            custom_size: Some(Vec2::splat(stats.size)),
            color: kind.color(),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 0.5),
    ));

    if kind == EnemyKind::Shooter {
        enemy.insert(RangedAttacker {
            timer: Timer::from_seconds(2.2, TimerMode::Repeating),
            bullet_speed: 320.0,
            bullet_damage: 8.0 * wave.enemy_damage_multiplier(),
            bullet_count: 1,
            spread: 0.0,
        });
    }
}

/// 移動パターンに従って敵を動かす
fn move_enemies(
    time: Res<Time>,
    player: Single<&Transform, With<Player>>,
    mut enemies: Query<(&mut Transform, &Enemy, &MovePattern), Without<Player>>,
) {
    let player_position = player.translation.truncate();
    let elapsed = time.elapsed_secs();

    for (mut transform, enemy, pattern) in &mut enemies {
        let position = transform.translation.truncate();
        let to_player = (player_position - position).normalize_or_zero();

        let direction = match *pattern {
            MovePattern::Chase => to_player,
            // プレイヤー方向を基準に、時間で振れる角度をずらして蛇行させる
            MovePattern::Zigzag { phase } => {
                let sway = (elapsed * ZIGZAG_FREQUENCY + phase).sin() * ZIGZAG_MAX_ANGLE;
                Vec2::from_angle(sway).rotate(to_player)
            }
            MovePattern::KeepDistance {
                preferred,
                strafe_dir,
            } => {
                let distance = player_position.distance(position);
                if distance > preferred + KEEP_DISTANCE_BAND {
                    to_player
                } else if distance < preferred - KEEP_DISTANCE_BAND {
                    -to_player
                } else {
                    // ちょうどいい距離なら、撃ちながらゆっくり回り込む
                    to_player.perp() * strafe_dir * 0.5
                }
            }
        };

        let mut position = position + direction * enemy.speed * time.delta_secs();

        // 後退する敵が壁の外に出ないよう、アリーナ内にとどめる
        let bound = Vec2::new(ARENA_WIDTH, ARENA_HEIGHT) / 2.0 - enemy.size / 2.0;
        position = position.clamp(-bound, bound);

        transform.translation = position.extend(transform.translation.z);
    }
}

/// 弾を撃つ敵（射撃型・ボス）の射撃タイマーを進め、プレイヤーに向けて弾を放つ
fn fire_ranged_attacks(
    mut commands: Commands,
    time: Res<Time>,
    player: Single<&Transform, With<Player>>,
    mut attackers: Query<(&Transform, &mut RangedAttacker), Without<Player>>,
) {
    let player_position = player.translation.truncate();

    for (transform, mut attacker) in &mut attackers {
        attacker.timer.tick(time.delta());
        if !attacker.timer.just_finished() {
            continue;
        }

        let origin = transform.translation.truncate();
        let base_angle = (player_position - origin).to_angle();
        let count = attacker.bullet_count;

        for i in 0..count {
            // 複数発は基準角の左右対称に spread 間隔で並べる（全周弾もこの式で表せる）
            let offset = attacker.spread * (i as f32 - (count as f32 - 1.0) / 2.0);
            let angle = base_angle + offset;

            commands.spawn((
                EnemyBullet {
                    velocity: Vec2::from_angle(angle) * attacker.bullet_speed,
                    damage: attacker.bullet_damage,
                    ttl: Timer::from_seconds(BULLET_TTL_SECS, TimerMode::Once),
                },
                // 専用画像がないため、単色の四角を45度傾けてひし形の弾にする
                Sprite::from_color(Color::srgb(1.0, 0.45, 0.3), Vec2::splat(BULLET_SIZE)),
                Transform::from_xyz(origin.x, origin.y, 0.65)
                    .with_rotation(Quat::from_rotation_z(angle + TAU / 8.0)),
            ));
        }
    }
}

/// 敵弾を直進させ、寿命切れかアリーナ外に出たものを消す
fn move_enemy_bullets(
    mut commands: Commands,
    time: Res<Time>,
    mut bullets: Query<(Entity, &mut Transform, &mut EnemyBullet)>,
) {
    for (entity, mut transform, mut bullet) in &mut bullets {
        bullet.ttl.tick(time.delta());
        transform.translation += (bullet.velocity * time.delta_secs()).extend(0.0);

        let position = transform.translation.truncate();
        let outside = position.x.abs() > ARENA_WIDTH / 2.0 || position.y.abs() > ARENA_HEIGHT / 2.0;
        if bullet.ttl.is_finished() || outside {
            commands.entity(entity).despawn();
        }
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

/// 敵弾がプレイヤーに当たったらダメージを与えて弾を消す（無敵時間中は無効）
fn bullet_hit_player(
    mut commands: Commands,
    mut player: Single<(&Transform, &mut Player, &mut Health)>,
    bullets: Query<(Entity, &Transform, &EnemyBullet)>,
) {
    let (player_transform, player, health) = &mut *player;

    if !player.invincible_timer.is_finished() {
        return;
    }

    let player_position = player_transform.translation.truncate();
    let hit_distance = (PLAYER_SIZE + BULLET_SIZE) / 2.0;

    for (entity, bullet_transform, bullet) in &bullets {
        let distance = player_position.distance(bullet_transform.translation.truncate());
        if distance < hit_distance {
            health.current -= bullet.damage;
            player.invincible_timer.reset();
            commands.entity(entity).despawn();
            break;
        }
    }
}

/// ラン終了時に残っている敵（ボス含む）をすべて消す
fn despawn_all_enemies(mut commands: Commands, enemies: Query<Entity, With<Enemy>>) {
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
}

/// 飛んでいる敵弾をすべて消す
fn despawn_enemy_bullets(mut commands: Commands, bullets: Query<Entity, With<EnemyBullet>>) {
    for entity in &bullets {
        commands.entity(entity).despawn();
    }
}

/// HPが0以下になった敵を消し、死亡メッセージを送る
fn despawn_dead_enemies(
    mut commands: Commands,
    mut died: MessageWriter<EnemyDied>,
    enemies: Query<(Entity, &Transform, &Health), With<Enemy>>,
) {
    for (entity, transform, health) in &enemies {
        if health.current <= 0.0 {
            died.write(EnemyDied {
                position: transform.translation.truncate(),
            });
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_waves_spawn_only_normal() {
        for roll in [0.0, 0.5, 0.99] {
            assert_eq!(pick_enemy_kind(1, roll), EnemyKind::Normal);
            assert_eq!(pick_enemy_kind(2, roll), EnemyKind::Normal);
        }
    }

    #[test]
    fn fast_appears_from_wave_3() {
        assert_eq!(pick_enemy_kind(3, 0.1), EnemyKind::Fast);
        assert_eq!(pick_enemy_kind(3, 0.5), EnemyKind::Normal);
    }

    #[test]
    fn tank_appears_from_wave_5() {
        assert_eq!(pick_enemy_kind(4, 0.1), EnemyKind::Fast);
        assert_eq!(pick_enemy_kind(5, 0.1), EnemyKind::Tank);
        assert_eq!(pick_enemy_kind(5, 0.3), EnemyKind::Fast);
        assert_eq!(pick_enemy_kind(5, 0.9), EnemyKind::Normal);
    }

    #[test]
    fn shooter_appears_from_wave_6() {
        // Wave 5 までは射撃型は出ない
        for roll in [0.0, 0.5, 0.99] {
            assert_ne!(pick_enemy_kind(5, roll), EnemyKind::Shooter);
        }
        assert_eq!(pick_enemy_kind(6, 0.1), EnemyKind::Shooter);
        assert_eq!(pick_enemy_kind(6, 0.2), EnemyKind::Tank);
        assert_eq!(pick_enemy_kind(6, 0.5), EnemyKind::Fast);
        assert_eq!(pick_enemy_kind(6, 0.9), EnemyKind::Normal);
        assert_eq!(pick_enemy_kind(10, 0.1), EnemyKind::Shooter);
    }
}
