use bevy::input::mouse::MouseButton;
use bevy::prelude::*;

use crate::input::confirm_just_pressed;
use crate::lang::{Language, UiFont};
use crate::{GameState, RunResult};

/// タイトル画面のUIルートに付けるマーカー
#[derive(Component)]
struct TitleScreen;

/// タイトル画面内の言語で変わるテキスト（毎フレーム現在の言語で更新する）
#[derive(Component)]
enum TitleText {
    Start,
    MoveHint,
    LangHint,
}

/// 言語切替ボタンに付けるマーカー（タップ/クリックで切替、ゲーム開始は誘発しない）
#[derive(Component)]
struct LangButton;

/// リザルト画面のUIルートに付けるマーカー
#[derive(Component)]
struct ResultScreen;

pub struct ScreensPlugin;

impl Plugin for ScreensPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Title), spawn_title_screen)
            .add_systems(
                Update,
                (update_title_texts, title_input).run_if(in_state(GameState::Title)),
            )
            .add_systems(OnExit(GameState::Title), despawn_title_screen)
            .add_systems(OnEnter(GameState::Result), spawn_result_screen)
            .add_systems(Update, result_input.run_if(in_state(GameState::Result)))
            .add_systems(OnExit(GameState::Result), despawn_result_screen);
    }
}

fn spawn_title_screen(mut commands: Commands, font: Res<UiFont>) {
    let text_font = |size: f32| TextFont {
        font: font.0.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    };

    commands.spawn((
        TitleScreen,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        children![
            (Text::new("BEBY ROGUE LIKE"), text_font(56.0)),
            (TitleText::Start, Text::new(""), text_font(24.0)),
            (TitleText::MoveHint, Text::new(""), text_font(18.0)),
            (
                LangButton,
                Button,
                BackgroundColor(Color::NONE),
                TitleText::LangHint,
                Text::new(""),
                text_font(18.0)
            ),
        ],
    ));
}

/// 現在の言語設定に合わせてタイトル画面のテキストを更新する
fn update_title_texts(lang: Res<Language>, mut texts: Query<(&TitleText, &mut Text)>) {
    for (kind, mut text) in &mut texts {
        text.0 = match kind {
            TitleText::Start => lang.press_start().to_string(),
            TitleText::MoveHint => lang.move_hint().to_string(),
            TitleText::LangHint => lang.lang_hint().to_string(),
        };
    }
}

/// スペース/タップでゲーム開始、Lキーまたは言語ボタンのタップで言語切替
fn title_input(
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    lang_button: Query<&Interaction, (Changed<Interaction>, With<LangButton>)>,
    mut lang: ResMut<Language>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut lang_toggled = false;
    for interaction in &lang_button {
        if *interaction == Interaction::Pressed {
            *lang = lang.toggle();
            lang_toggled = true;
        }
    }
    if keys.just_pressed(KeyCode::KeyL) {
        *lang = lang.toggle();
        lang_toggled = true;
    }

    // 言語ボタンをタップしたフレームはゲーム開始判定をスキップする
    if !lang_toggled && confirm_just_pressed(&keys, &touches, &mouse) {
        next_state.set(GameState::Playing);
    }
}

fn despawn_title_screen(mut commands: Commands, screens: Query<Entity, With<TitleScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}

fn spawn_result_screen(
    mut commands: Commands,
    font: Res<UiFont>,
    lang: Res<Language>,
    result: Res<RunResult>,
) {
    let text_font = |size: f32| TextFont {
        font: font.0.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    };

    let heading = if result.victory {
        lang.victory()
    } else {
        lang.game_over()
    };
    let heading_color = if result.victory {
        Color::srgb(1.0, 0.85, 0.3)
    } else {
        Color::srgb(0.9, 0.3, 0.3)
    };

    let weapons_line = result
        .weapons
        .iter()
        .map(|(weapon_type, level)| format!("{} Lv{}", lang.weapon_name(*weapon_type), level))
        .collect::<Vec<_>>()
        .join("   ");

    commands.spawn((
        ResultScreen,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        children![
            (Text::new(heading), text_font(48.0), TextColor(heading_color)),
            (Text::new(lang.reached_wave(result.wave_reached)), text_font(24.0)),
            (Text::new(lang.score(result.score)), text_font(24.0)),
            (Text::new(weapons_line), text_font(20.0)),
            (Text::new(lang.press_return_title()), text_font(18.0)),
        ],
    ));
}

/// スペース/タップでタイトルへ戻る
fn result_input(
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if confirm_just_pressed(&keys, &touches, &mouse) {
        next_state.set(GameState::Title);
    }
}

fn despawn_result_screen(mut commands: Commands, screens: Query<Entity, With<ResultScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}
