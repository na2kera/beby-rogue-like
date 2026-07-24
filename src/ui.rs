use bevy::prelude::*;

use crate::lang::{Language, UiFont};
use crate::player::Player;
use crate::score::Score;
use crate::wave::WaveInfo;
use crate::weapon::Weapon;
use crate::{GameState, Health};

/// HUD（プレイ中の常時表示UI）のルートに付けるマーカー
#[derive(Component)]
struct Hud;

/// HP表示テキストのマーカーコンポーネント
#[derive(Component)]
struct HpText;

/// 所持武器一覧テキストのマーカーコンポーネント
#[derive(Component)]
struct WeaponText;

/// ウェーブ番号・残り時間テキストのマーカーコンポーネント
#[derive(Component)]
struct WaveText;

/// スコア表示テキストのマーカーコンポーネント
#[derive(Component)]
struct ScoreText;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_hud)
            .add_systems(
                Update,
                (
                    update_hp_text,
                    update_weapon_text,
                    update_wave_text,
                    update_score_text,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), despawn_hud);
    }
}

fn spawn_hud(mut commands: Commands, font: Res<UiFont>) {
    let text_font = TextFont {
        font: font.0.clone().into(),
        font_size: FontSize::Px(20.0),
        ..default()
    };

    commands.spawn((
        Hud,
        HpText,
        Text::new("HP: -"),
        text_font.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Hud,
        WeaponText,
        Text::new(""),
        text_font.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    // 画面上部中央にウェーブ表示（横幅いっぱいのコンテナで中央寄せ）
    commands.spawn((
        Hud,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(WaveText, Text::new(""), text_font.clone())],
    ));

    // 画面右上にスコア表示
    commands.spawn((
        Hud,
        ScoreText,
        Text::new(""),
        text_font,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
    ));
}

fn despawn_hud(mut commands: Commands, huds: Query<Entity, With<Hud>>) {
    for entity in &huds {
        commands.entity(entity).despawn();
    }
}

fn update_hp_text(
    player: Single<&Health, With<Player>>,
    mut text: Single<&mut Text, With<HpText>>,
) {
    text.0 = format!("HP: {:.0} / {:.0}", player.current.max(0.0), player.max);
}

fn update_score_text(
    lang: Res<Language>,
    score: Res<Score>,
    mut text: Single<&mut Text, With<ScoreText>>,
) {
    text.0 = lang.score(score.0);
}

fn update_wave_text(wave: Res<WaveInfo>, mut text: Single<&mut Text, With<WaveText>>) {
    text.0 = if wave.is_boss_wave() {
        format!("Wave {}   BOSS", wave.number)
    } else {
        format!(
            "Wave {}   {:.0}s",
            wave.number,
            wave.timer.remaining_secs().ceil()
        )
    };
}

fn update_weapon_text(
    lang: Res<Language>,
    weapons: Query<&Weapon>,
    mut text: Single<&mut Text, With<WeaponText>>,
) {
    let lines: Vec<String> = weapons
        .iter()
        .map(|w| format!("{} Lv{}", lang.weapon_name(w.weapon_type), w.level))
        .collect();
    text.0 = lines.join("\n");
}
