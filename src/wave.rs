use bevy::input::mouse::MouseButton;
use bevy::prelude::*;

use crate::assets::SpriteAssets;
use crate::enemy::{Boss, Enemy};
use crate::input::confirm_just_pressed;
use crate::lang::{Language, UiFont};
use crate::weapon::Weapon;
use crate::{ARENA_HEIGHT, GameState, Health, RunResult};

/// 最終ウェーブ番号（最終ボスが出る）
pub const FINAL_WAVE: u32 = 10;

/// 中ボスが出るウェーブ番号
pub const MID_BOSS_WAVE: u32 = 5;

/// ウェーブの進行状態。GameState::Playing の間だけ存在するサブステート。
/// Fighting = 戦闘中、Intermission = ウェーブ間の結果確認画面
#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Playing)]
pub enum WavePhase {
    #[default]
    Fighting,
    Intermission,
}

/// 現在のウェーブ番号と残り時間
#[derive(Resource)]
pub struct WaveInfo {
    pub number: u32,
    pub timer: Timer,
    /// 最終ウェーブでボスを出現させたか（勝利判定に使う）
    pub boss_spawned: bool,
}

impl WaveInfo {
    pub fn new(number: u32) -> Self {
        // ウェーブが進むほど長くなる: Wave1=30秒 → Wave10=57秒
        let duration = 30.0 + 3.0 * (number - 1) as f32;
        Self {
            number,
            timer: Timer::from_seconds(duration, TimerMode::Once),
            boss_spawned: false,
        }
    }

    /// 敵のHP倍率（ウェーブが進むほど硬くなる）
    pub fn enemy_hp_multiplier(&self) -> f32 {
        1.0 + 0.15 * (self.number - 1) as f32
    }

    /// 敵の出現間隔（ウェーブが進むほど短くなる）
    pub fn spawn_interval_secs(&self) -> f32 {
        (1.2 * 0.92f32.powi(self.number as i32 - 1)).max(0.4)
    }

    /// 敵の接触ダメージ倍率（ウェーブが進むほど痛くなる）
    pub fn enemy_damage_multiplier(&self) -> f32 {
        1.0 + 0.10 * (self.number - 1) as f32
    }

    /// 敵の移動速度倍率（ウェーブが進むほど速くなる。速すぎると理不尽なので上限あり）
    pub fn enemy_speed_multiplier(&self) -> f32 {
        (1.0 + 0.03 * (self.number - 1) as f32).min(1.3)
    }

    /// ボスウェーブかどうか。
    /// ボスウェーブは時間制ではなく、敵を全滅させると次へ進める
    pub fn is_boss_wave(&self) -> bool {
        self.number == MID_BOSS_WAVE || self.number >= FINAL_WAVE
    }
}

/// 開発用: `START_WAVE=10 cargo run` のように開始ウェーブを指定できる
fn start_wave_number() -> u32 {
    std::env::var("START_WAVE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

/// ウェーブ間リザルト画面のUIに付けるマーカー
#[derive(Component)]
struct IntermissionScreen;

pub struct WavePlugin;

impl Plugin for WavePlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<WavePhase>()
            .insert_resource(WaveInfo::new(start_wave_number()))
            .add_systems(OnEnter(GameState::Playing), reset_wave)
            .add_systems(OnEnter(WavePhase::Fighting), spawn_boss_on_boss_wave)
            .add_systems(
                Update,
                (tick_wave, check_boss_wave_cleared).run_if(in_state(WavePhase::Fighting)),
            )
            .add_systems(
                OnEnter(WavePhase::Intermission),
                (clear_enemies, spawn_intermission_screen),
            )
            .add_systems(
                Update,
                advance_wave.run_if(in_state(WavePhase::Intermission)),
            )
            .add_systems(OnExit(WavePhase::Intermission), despawn_intermission_screen);
    }
}

/// ラン開始時にウェーブを最初から初期化する
fn reset_wave(mut wave: ResMut<WaveInfo>) {
    *wave = WaveInfo::new(start_wave_number());
}

/// ウェーブの残り時間を進め、時間切れでリザルトへ移行する。
/// ボスウェーブ（5と10）だけは時間制ではなく「敵を全滅させたらクリア」
fn tick_wave(
    time: Res<Time>,
    mut wave: ResMut<WaveInfo>,
    mut next_phase: ResMut<NextState<WavePhase>>,
) {
    if wave.is_boss_wave() {
        return;
    }
    wave.timer.tick(time.delta());
    if wave.timer.is_finished() {
        next_phase.set(WavePhase::Intermission);
    }
}

/// ボスウェーブの開始時にボスを1体出現させる。
/// Wave 5 は中ボス、Wave 10 は最終ボス
fn spawn_boss_on_boss_wave(
    mut commands: Commands,
    sprites: Res<SpriteAssets>,
    mut wave: ResMut<WaveInfo>,
) {
    if !wave.is_boss_wave() || wave.boss_spawned {
        return;
    }
    wave.boss_spawned = true;

    let (size, speed, contact_damage, max_hp) = if wave.number >= FINAL_WAVE {
        (192.0, 70.0, 30.0, 1500.0)
    } else {
        (144.0, 85.0, 20.0, 600.0)
    };

    commands.spawn((
        Boss,
        Enemy {
            size,
            speed,
            contact_damage,
        },
        Health::new(max_hp),
        Sprite {
            image: sprites.boss.clone(),
            custom_size: Some(Vec2::splat(size)),
            ..default()
        },
        Transform::from_xyz(0.0, ARENA_HEIGHT / 2.0 - 200.0, 0.6),
    ));
}

/// ボスウェーブで敵（ボス含む）を全滅させたら次へ進む。
/// 最終ウェーブなら勝利、中ボスウェーブならウェーブ間リザルトへ
fn check_boss_wave_cleared(
    mut commands: Commands,
    wave: Res<WaveInfo>,
    enemies: Query<(), With<Enemy>>,
    weapons: Query<&Weapon>,
    mut next_state: ResMut<NextState<GameState>>,
    mut next_phase: ResMut<NextState<WavePhase>>,
) {
    if !wave.is_boss_wave() || !wave.boss_spawned || !enemies.is_empty() {
        return;
    }

    if wave.number >= FINAL_WAVE {
        commands.insert_resource(RunResult {
            victory: true,
            wave_reached: wave.number,
            weapons: weapons.iter().map(|w| (w.weapon_type, w.level)).collect(),
        });
        next_state.set(GameState::Result);
    } else {
        next_phase.set(WavePhase::Intermission);
    }
}

/// ウェーブ終了時に生き残っている敵を一掃する
fn clear_enemies(mut commands: Commands, enemies: Query<Entity, With<Enemy>>) {
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
}

/// ウェーブ間の結果確認画面（獲得武器の一覧）を表示する
fn spawn_intermission_screen(
    mut commands: Commands,
    font: Res<UiFont>,
    lang: Res<Language>,
    wave: Res<WaveInfo>,
    weapons: Query<&Weapon>,
) {
    let mut lines = vec![lang.wave_cleared(wave.number), String::new()];
    for weapon in &weapons {
        lines.push(format!(
            "{} Lv{}",
            lang.weapon_name(weapon.weapon_type),
            weapon.level
        ));
    }
    lines.push(String::new());
    lines.push(lang.press_next_wave().to_string());

    // 画面全体を覆う半透明の黒背景の中央にテキストを置く
    commands.spawn((
        IntermissionScreen,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        children![(
            Text::new(lines.join("\n")),
            TextFont {
                font: font.0.clone().into(),
                font_size: FontSize::Px(24.0),
                ..default()
            }
        )],
    ));
}

/// スペースキー / タップで次のウェーブを開始する
fn advance_wave(
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wave: ResMut<WaveInfo>,
    mut next_phase: ResMut<NextState<WavePhase>>,
) {
    if confirm_just_pressed(&keys, &touches, &mouse) {
        *wave = WaveInfo::new(wave.number + 1);
        next_phase.set(WavePhase::Fighting);
    }
}

fn despawn_intermission_screen(
    mut commands: Commands,
    screens: Query<Entity, With<IntermissionScreen>>,
) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_grows_with_wave_number() {
        let wave1 = WaveInfo::new(1);
        let wave10 = WaveInfo::new(FINAL_WAVE);

        // Wave 1 はすべて等倍
        assert_eq!(wave1.enemy_hp_multiplier(), 1.0);
        assert_eq!(wave1.enemy_damage_multiplier(), 1.0);
        assert_eq!(wave1.enemy_speed_multiplier(), 1.0);

        // Wave 10 では HP 2.35倍・攻撃力 1.9倍・速度 1.27倍
        assert!((wave10.enemy_hp_multiplier() - 2.35).abs() < 1e-5);
        assert!((wave10.enemy_damage_multiplier() - 1.9).abs() < 1e-5);
        assert!((wave10.enemy_speed_multiplier() - 1.27).abs() < 1e-5);

        // 出現間隔はウェーブが進むほど短くなり、下限を下回らない
        assert!(wave10.spawn_interval_secs() < wave1.spawn_interval_secs());
        assert!(wave10.spawn_interval_secs() >= 0.4);
    }

    #[test]
    fn speed_multiplier_is_capped() {
        // 想定外に大きいウェーブ番号でも速度は 1.3 倍で頭打ち
        assert_eq!(WaveInfo::new(100).enemy_speed_multiplier(), 1.3);
    }

    #[test]
    fn boss_waves_are_5_and_10() {
        assert!(WaveInfo::new(MID_BOSS_WAVE).is_boss_wave());
        assert!(WaveInfo::new(FINAL_WAVE).is_boss_wave());
        assert!(!WaveInfo::new(4).is_boss_wave());
        assert!(!WaveInfo::new(6).is_boss_wave());
    }
}
