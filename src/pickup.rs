use bevy::prelude::*;
use rand::RngExt;

use crate::Health;
use crate::assets::SpriteAssets;
use crate::enemy::EnemyDied;
use crate::player::{PLAYER_SIZE, Player};
use crate::wave::WavePhase;
use crate::weapon::{MAX_LEVEL, MAX_WEAPONS, Weapon, WeaponType};
use crate::{ARENA_HEIGHT, ARENA_WIDTH, GameState};

const PICKUP_SIZE: f32 = 24.0;

/// 敵1体あたりの武器ドロップ率と回復アイテムドロップ率
const WEAPON_DROP_RATE: f32 = 0.10;
const HEAL_DROP_RATE: f32 = 0.06;

/// ウェーブ開始時にフィールドへ置く武器の個数
const FIELD_WEAPONS_PER_WAVE: u32 = 1;

/// フィールド配置時に壁際を避けるマージン
const FIELD_SPAWN_MARGIN: f32 = 100.0;

/// フィールド配置の武器はプレイヤーからこの距離範囲（リング状）に出す。
/// 画面の縦方向の視界が±360pxなので、最大350なら必ず画面内に収まる
const FIELD_SPAWN_MIN_DISTANCE: f32 = 150.0;
const FIELD_SPAWN_MAX_DISTANCE: f32 = 350.0;

const HEAL_AMOUNT: f32 = 20.0;

/// フィールドに落ちている拾得アイテム
#[derive(Component)]
struct Pickup(PickupKind);

#[derive(Clone, Copy)]
enum PickupKind {
    Weapon(WeaponType),
    Heal,
}

pub struct PickupPlugin;

impl Plugin for PickupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WavePhase::Fighting), scatter_field_weapons)
            .add_systems(
                Update,
                (spawn_drops, collect_pickups)
                    .chain()
                    .run_if(in_state(WavePhase::Fighting)),
            )
            .add_systems(OnExit(GameState::Playing), despawn_all_pickups);
    }
}

/// ドロップ候補 = まだ強化余地のある所持武器 ＋（枠が空いていれば）未所持武器。
/// 枠が満杯なら新武器は候補から外れる＝「新武器は出なくなる」仕様
fn weapon_candidates(weapons: &Query<&Weapon>) -> Vec<WeaponType> {
    let owned_count = weapons.iter().count();
    let mut candidates = Vec::new();
    for weapon_type in WeaponType::ALL {
        match weapons.iter().find(|w| w.weapon_type == weapon_type) {
            Some(w) if w.level < MAX_LEVEL => candidates.push(weapon_type),
            None if owned_count < MAX_WEAPONS => candidates.push(weapon_type),
            _ => {}
        }
    }
    candidates
}

/// ウェーブ開始時に、プレイヤー周辺のリング状の範囲へ武器を数個ばら撒く
fn scatter_field_weapons(
    mut commands: Commands,
    sprites: Res<SpriteAssets>,
    weapons: Query<&Weapon>,
    player: Query<&Transform, With<Player>>,
) {
    let candidates = weapon_candidates(&weapons);
    if candidates.is_empty() {
        return;
    }

    // Wave 1 ではプレイヤーがまだスポーンしていないことがあるため、
    // その場合は初期位置（原点）を中心にする
    let center = player
        .single()
        .map(|transform| transform.translation.truncate())
        .unwrap_or(Vec2::ZERO);

    let half = Vec2::new(
        ARENA_WIDTH / 2.0 - FIELD_SPAWN_MARGIN,
        ARENA_HEIGHT / 2.0 - FIELD_SPAWN_MARGIN,
    );

    let mut rng = rand::rng();
    for _ in 0..FIELD_WEAPONS_PER_WAVE {
        let weapon_type = candidates[rng.random_range(0..candidates.len())];
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let distance = rng.random_range(FIELD_SPAWN_MIN_DISTANCE..FIELD_SPAWN_MAX_DISTANCE);
        let position = (center + Vec2::from_angle(angle) * distance).clamp(-half, half);
        commands.spawn((
            Pickup(PickupKind::Weapon(weapon_type)),
            Sprite {
                image: sprites.weapon_drop_icon(weapon_type),
                custom_size: Some(Vec2::splat(PICKUP_SIZE)),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, 0.3),
        ));
    }
}

/// 敵の死亡メッセージを受け取り、確率でアイテムを落とす
fn spawn_drops(
    mut commands: Commands,
    mut died: MessageReader<EnemyDied>,
    sprites: Res<SpriteAssets>,
    weapons: Query<&Weapon>,
) {
    let candidates = weapon_candidates(&weapons);

    let mut rng = rand::rng();
    for event in died.read() {
        let roll: f32 = rng.random();
        let kind = if roll < WEAPON_DROP_RATE && !candidates.is_empty() {
            PickupKind::Weapon(candidates[rng.random_range(0..candidates.len())])
        } else if roll < WEAPON_DROP_RATE + HEAL_DROP_RATE {
            PickupKind::Heal
        } else {
            continue;
        };

        let image = match kind {
            PickupKind::Weapon(weapon_type) => sprites.weapon_drop_icon(weapon_type),
            PickupKind::Heal => sprites.drop_heal.clone(),
        };

        commands.spawn((
            Pickup(kind),
            Sprite {
                image,
                custom_size: Some(Vec2::splat(PICKUP_SIZE)),
                ..default()
            },
            Transform::from_xyz(event.position.x, event.position.y, 0.3),
        ));
    }
}

/// ラン終了時に残っているアイテムをすべて消す
fn despawn_all_pickups(mut commands: Commands, pickups: Query<Entity, With<Pickup>>) {
    for entity in &pickups {
        commands.entity(entity).despawn();
    }
}

/// プレイヤーがアイテムに触れたら効果を適用して消す
fn collect_pickups(
    mut commands: Commands,
    mut player: Single<(&Transform, &mut Health), With<Player>>,
    pickups: Query<(Entity, &Transform, &Pickup)>,
    mut weapons: Query<&mut Weapon>,
) {
    let (player_transform, health) = &mut *player;
    let player_position = player_transform.translation.truncate();
    let pickup_distance = (PLAYER_SIZE + PICKUP_SIZE) / 2.0;

    // 同一フレームで同じ新武器を2回スポーンしないためのガード
    let mut newly_added: Vec<WeaponType> = Vec::new();

    for (entity, transform, pickup) in &pickups {
        if player_position.distance(transform.translation.truncate()) >= pickup_distance {
            continue;
        }

        match pickup.0 {
            PickupKind::Heal => {
                health.current = (health.current + HEAL_AMOUNT).min(health.max);
            }
            PickupKind::Weapon(weapon_type) => {
                if let Some(mut weapon) =
                    weapons.iter_mut().find(|w| w.weapon_type == weapon_type)
                {
                    // 所持済みならレベルアップ（強化）
                    if weapon.level < MAX_LEVEL {
                        weapon.level += 1;
                    }
                } else if weapons.iter().count() + newly_added.len() < MAX_WEAPONS
                    && !newly_added.contains(&weapon_type)
                {
                    // 未所持で枠が空いていれば新しい武器として追加
                    commands.spawn(Weapon::new(weapon_type));
                    newly_added.push(weapon_type);
                }
            }
        }

        commands.entity(entity).despawn();
    }
}
