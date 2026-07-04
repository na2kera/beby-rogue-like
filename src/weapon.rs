use bevy::prelude::*;

use crate::enemy::Enemy;
use crate::player::Player;
use crate::wave::WavePhase;
use crate::{GameState, Health};

/// 同時に持てる武器の最大数
pub const MAX_WEAPONS: usize = 6;
/// 武器レベルの上限
pub const MAX_LEVEL: u8 = 8;

/// 弾の当たり判定の大きさ（見た目より少し大きめ）
const PROJECTILE_HIT_SIZE: f32 = 12.0;

/// 武器の種類
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponType {
    /// 周囲なぎ払い
    Sword,
    /// 最寄りの敵への直線弾
    Bow,
    /// 貫通弾
    Spear,
    /// 自分周りの持続ダメージ
    Aura,
    /// 範囲爆発
    Bomb,
}

impl WeaponType {
    /// 全武器の一覧（ドロップ抽選に使う）
    pub const ALL: [WeaponType; 5] = [
        WeaponType::Sword,
        WeaponType::Bow,
        WeaponType::Spear,
        WeaponType::Aura,
        WeaponType::Bomb,
    ];

    /// 発動間隔（秒）
    fn cooldown_secs(self) -> f32 {
        match self {
            WeaponType::Sword => 1.0,
            WeaponType::Bow => 0.8,
            WeaponType::Spear => 1.5,
            WeaponType::Aura => 0.5,
            WeaponType::Bomb => 2.5,
        }
    }
}

/// 武器1つ分のエンティティに付くコンポーネント。
/// プレイヤーとは別のエンティティとして存在し、最大6つまで持てる
#[derive(Component)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub level: u8,
    cooldown: Timer,
}

impl Weapon {
    pub fn new(weapon_type: WeaponType) -> Self {
        Self {
            weapon_type,
            level: 1,
            cooldown: Timer::from_seconds(weapon_type.cooldown_secs(), TimerMode::Repeating),
        }
    }
}

/// まっすぐ飛ぶ弾（矢・槍）。pierce = true なら敵を貫通する
#[derive(Component)]
struct Projectile {
    damage: f32,
    velocity: Vec2,
    pierce: bool,
    /// 貫通弾が同じ敵に何度も当たらないようにするための記録
    already_hit: Vec<Entity>,
}

/// 目標地点まで飛んで爆発するボム
#[derive(Component)]
struct BombProjectile {
    damage: f32,
    explosion_radius: f32,
    target: Vec2,
    speed: f32,
}

/// 一定時間たったら自動で消えるエンティティ（エフェクトや弾の射程に使う）
#[derive(Component)]
struct Lifetime(Timer);

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_initial_weapon)
            .add_systems(
                Update,
                (fire_weapons, move_projectiles, move_bombs)
                    .chain()
                    .run_if(in_state(WavePhase::Fighting)),
            )
            .add_systems(Update, despawn_expired)
            .add_systems(OnExit(GameState::Playing), cleanup_weapons);
    }
}

/// 初期武器として剣を1本持たせる
fn spawn_initial_weapon(mut commands: Commands) {
    commands.spawn(Weapon::new(WeaponType::Sword));
}

/// プレイヤーから最も近い敵の位置を返す
fn nearest_enemy_position(
    from: Vec2,
    enemies: &Query<(Entity, &Transform, &Enemy, &mut Health)>,
) -> Option<Vec2> {
    enemies
        .iter()
        .map(|(_, transform, _, _)| transform.translation.truncate())
        .min_by(|a, b| from.distance(*a).total_cmp(&from.distance(*b)))
}

/// 全武器のクールダウンを進め、発動タイミングが来た武器を撃つ
fn fire_weapons(
    mut commands: Commands,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    player: Single<&Transform, With<Player>>,
    mut weapons: Query<&mut Weapon>,
    mut enemies: Query<(Entity, &Transform, &Enemy, &mut Health)>,
) {
    let player_position = player.translation.truncate();

    for mut weapon in &mut weapons {
        weapon.cooldown.tick(time.delta());
        if !weapon.cooldown.just_finished() {
            continue;
        }

        // レベルアップ1回ごとの上昇量を計算しやすいよう「レベル-1」を使う
        let bonus = (weapon.level - 1) as f32;

        match weapon.weapon_type {
            // 剣・オーラ: プレイヤー周囲の敵全員に即時ダメージ＋円エフェクト
            WeaponType::Sword | WeaponType::Aura => {
                let (damage, radius, color) = match weapon.weapon_type {
                    WeaponType::Sword => (
                        15.0 + 5.0 * bonus,
                        120.0 + 10.0 * bonus,
                        Color::srgba(1.0, 0.9, 0.3, 0.25), // 黄
                    ),
                    _ => (
                        5.0 + 2.0 * bonus,
                        90.0 + 8.0 * bonus,
                        Color::srgba(0.6, 0.3, 0.9, 0.2), // 紫
                    ),
                };

                for (_, enemy_transform, _, mut health) in &mut enemies {
                    let distance =
                        player_position.distance(enemy_transform.translation.truncate());
                    if distance <= radius {
                        health.current -= damage;
                    }
                }

                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(radius))),
                    MeshMaterial2d(materials.add(color)),
                    Transform::from_xyz(player_position.x, player_position.y, 0.8),
                    Lifetime(Timer::from_seconds(0.15, TimerMode::Once)),
                ));
            }

            // 弓: 最寄りの敵に向かって矢を放つ（レベルが上がると本数が増える）
            WeaponType::Bow => {
                let Some(target) = nearest_enemy_position(player_position, &enemies) else {
                    continue;
                };
                let damage = 12.0 + 4.0 * bonus;
                let arrow_count = 1 + weapon.level as i32 / 4; // Lv4で2本、Lv8で3本
                let base_angle = (target - player_position).to_angle();

                for i in 0..arrow_count {
                    // 複数本のときは少しずつ角度をずらして扇状に撃つ
                    let offset = 0.15 * (i - (arrow_count - 1) / 2) as f32;
                    let velocity = Vec2::from_angle(base_angle + offset) * 500.0;
                    spawn_projectile(
                        &mut commands,
                        player_position,
                        velocity,
                        damage,
                        false,
                        Color::srgb(0.8, 0.95, 1.0), // 水色
                        Vec2::new(18.0, 5.0),
                    );
                }
            }

            // 槍: 最寄りの敵に向かって貫通弾を放つ
            WeaponType::Spear => {
                let Some(target) = nearest_enemy_position(player_position, &enemies) else {
                    continue;
                };
                let damage = 10.0 + 4.0 * bonus;
                let velocity = (target - player_position).normalize_or_zero() * 600.0;
                spawn_projectile(
                    &mut commands,
                    player_position,
                    velocity,
                    damage,
                    true,
                    Color::srgb(0.6, 0.95, 0.6), // 黄緑
                    Vec2::new(28.0, 6.0),
                );
            }

            // ボム: 最寄りの敵の位置めがけて投げ、着弾点で範囲爆発
            WeaponType::Bomb => {
                let Some(target) = nearest_enemy_position(player_position, &enemies) else {
                    continue;
                };
                commands.spawn((
                    BombProjectile {
                        damage: 25.0 + 8.0 * bonus,
                        explosion_radius: 110.0 + 10.0 * bonus,
                        target,
                        speed: 400.0,
                    },
                    Sprite::from_color(Color::srgb(0.35, 0.35, 0.35), Vec2::splat(16.0)),
                    Transform::from_xyz(player_position.x, player_position.y, 0.7),
                ));
            }
        }
    }
}

/// 弾エンティティを1つ生成する（矢・槍で共用）
fn spawn_projectile(
    commands: &mut Commands,
    position: Vec2,
    velocity: Vec2,
    damage: f32,
    pierce: bool,
    color: Color,
    size: Vec2,
) {
    commands.spawn((
        Projectile {
            damage,
            velocity,
            pierce,
            already_hit: Vec::new(),
        },
        Sprite::from_color(color, size),
        Transform::from_xyz(position.x, position.y, 0.7)
            .with_rotation(Quat::from_rotation_z(velocity.to_angle())),
        // 射程の代わり: 2秒で消える
        Lifetime(Timer::from_seconds(2.0, TimerMode::Once)),
    ));
}

/// 弾を直進させ、敵に当たったらダメージを与える
fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile)>,
    mut enemies: Query<(Entity, &Transform, &Enemy, &mut Health), Without<Projectile>>,
) {
    for (projectile_entity, mut transform, mut projectile) in &mut projectiles {
        transform.translation += (projectile.velocity * time.delta_secs()).extend(0.0);
        let position = transform.translation.truncate();

        for (enemy_entity, enemy_transform, enemy, mut health) in &mut enemies {
            let hit_distance = (enemy.size + PROJECTILE_HIT_SIZE) / 2.0;
            if position.distance(enemy_transform.translation.truncate()) >= hit_distance {
                continue;
            }
            if projectile.already_hit.contains(&enemy_entity) {
                continue;
            }

            health.current -= projectile.damage;

            if projectile.pierce {
                projectile.already_hit.push(enemy_entity);
            } else {
                commands.entity(projectile_entity).despawn();
                break;
            }
        }
    }
}

/// ボムを目標地点まで飛ばし、到達したら爆発させる
fn move_bombs(
    mut commands: Commands,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut bombs: Query<(Entity, &mut Transform, &BombProjectile)>,
    mut enemies: Query<(&Transform, &Enemy, &mut Health), Without<BombProjectile>>,
) {
    for (bomb_entity, mut transform, bomb) in &mut bombs {
        let position = transform.translation.truncate();
        let step = bomb.speed * time.delta_secs();

        // 目標までの残り距離が1フレーム分以下なら着弾
        if position.distance(bomb.target) > step {
            let direction = (bomb.target - position).normalize_or_zero();
            transform.translation += (direction * step).extend(0.0);
            continue;
        }

        // 爆発: 範囲内の敵全員にダメージ
        for (enemy_transform, _, mut health) in &mut enemies {
            let distance = bomb.target.distance(enemy_transform.translation.truncate());
            if distance <= bomb.explosion_radius {
                health.current -= bomb.damage;
            }
        }

        commands.spawn((
            Mesh2d(meshes.add(Circle::new(bomb.explosion_radius))),
            MeshMaterial2d(materials.add(Color::srgba(1.0, 0.5, 0.1, 0.4))),
            Transform::from_xyz(bomb.target.x, bomb.target.y, 0.8),
            Lifetime(Timer::from_seconds(0.2, TimerMode::Once)),
        ));
        commands.entity(bomb_entity).despawn();
    }
}

/// ラン終了時に武器・弾・エフェクトをすべて消す
#[allow(clippy::type_complexity)]
fn cleanup_weapons(
    mut commands: Commands,
    entities: Query<
        Entity,
        Or<(
            With<Weapon>,
            With<Projectile>,
            With<BombProjectile>,
            With<Lifetime>,
        )>,
    >,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

/// Lifetime が切れたエンティティを消す
fn despawn_expired(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in &mut query {
        lifetime.0.tick(time.delta());
        if lifetime.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
