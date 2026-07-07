use bevy::input::mouse::MouseButton;
use bevy::prelude::*;

use crate::wave::WavePhase;

/// MoveInput を書き込む入力系システム群。移動処理はこのセットの後に実行する
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GameInputSet;

/// スティックの土台の半径（この範囲内でノブが動く）
const STICK_RADIUS: f32 = 60.0;
/// ノブ（つまみ）の見た目の半径
const KNOB_RADIUS: f32 = 24.0;

/// 現在フレームの移動方向（正規化済み）。キーボードとスティックの両方から書き込まれる
#[derive(Resource, Default)]
pub struct MoveInput(pub Vec2);

/// 仮想スティックを操作中のポインタ
#[derive(Clone, Copy, PartialEq)]
enum PointerSource {
    Touch(u64),
    Mouse,
}

/// 操作中のスティックの状態（原点とポインタ種別）
#[derive(Resource, Default)]
struct ActiveStick(Option<ActiveStickData>);

struct ActiveStickData {
    pointer: PointerSource,
    origin: Vec2,
}

/// スティックの土台UIに付けるマーカー
#[derive(Component)]
struct StickBase;

/// スティックのノブUIに付けるマーカー
#[derive(Component)]
struct StickKnob;

pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoveInput>()
            .init_resource::<ActiveStick>()
            .add_systems(
                Update,
                (keyboard_move_input, virtual_stick_input)
                    .chain()
                    .in_set(GameInputSet)
                    .run_if(in_state(WavePhase::Fighting)),
            )
            .add_systems(OnExit(WavePhase::Fighting), clear_stick);
    }
}

/// WASD / 矢印キーの入力を MoveInput に書き込む。押されていればスティックより優先される
fn keyboard_move_input(keys: Res<ButtonInput<KeyCode>>, mut move_input: ResMut<MoveInput>) {
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

    move_input.0 = direction.normalize_or_zero();
}

/// タッチ/マウスドラッグでフローティング仮想スティックを操作する。
/// キーボード入力があればそちらを優先し、この関数は上書きしない
#[allow(clippy::too_many_arguments)]
fn virtual_stick_input(
    mut commands: Commands,
    windows: Query<&Window>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut active: ResMut<ActiveStick>,
    mut move_input: ResMut<MoveInput>,
    mut knob_query: Query<&mut Node, With<StickKnob>>,
    stick_entities: Query<Entity, With<StickBase>>,
) {
    // キーボードで移動中はスティックを無視する（すでに MoveInput は書き込み済み）
    let keyboard_active = move_input.0 != Vec2::ZERO;

    let Ok(window) = windows.single() else {
        return;
    };

    // スティック開始: タッチまたはマウス押下
    if active.0.is_none() && !keyboard_active {
        if let Some(touch) = touches.iter_just_pressed().next() {
            spawn_stick(&mut commands, touch.position());
            active.0 = Some(ActiveStickData {
                pointer: PointerSource::Touch(touch.id()),
                origin: touch.position(),
            });
        } else if mouse.just_pressed(MouseButton::Left)
            && let Some(position) = window.cursor_position()
        {
            spawn_stick(&mut commands, position);
            active.0 = Some(ActiveStickData {
                pointer: PointerSource::Mouse,
                origin: position,
            });
        }
    }

    // スティック操作中の更新
    if let Some(data) = &active.0 {
        let current_position = match data.pointer {
            PointerSource::Touch(id) => touches.get_pressed(id).map(|t| t.position()),
            PointerSource::Mouse => window.cursor_position(),
        };

        let released = match data.pointer {
            PointerSource::Touch(id) => touches.get_pressed(id).is_none(),
            PointerSource::Mouse => mouse.just_released(MouseButton::Left),
        };

        if released || current_position.is_none() {
            despawn_stick(&mut commands, &stick_entities);
            active.0 = None;
            if !keyboard_active {
                move_input.0 = Vec2::ZERO;
            }
            return;
        }

        let current_position = current_position.unwrap();
        let delta = current_position - data.origin;
        let clamped = delta.clamp_length_max(STICK_RADIUS);

        // ノブの見た目位置を更新（土台中心からのオフセット）
        if let Ok(mut knob_node) = knob_query.single_mut() {
            knob_node.left = Val::Px(STICK_RADIUS - KNOB_RADIUS + clamped.x);
            knob_node.top = Val::Px(STICK_RADIUS - KNOB_RADIUS + clamped.y);
        }

        if !keyboard_active {
            // 画面座標はY下向きなので、ゲーム内の上方向(+Y)に合わせて反転する
            let direction = Vec2::new(clamped.x, -clamped.y) / STICK_RADIUS;
            move_input.0 = direction.clamp_length_max(1.0);
        }
    }
}

/// スティックUI（土台+ノブ）を指定位置を中心に生成する
fn spawn_stick(commands: &mut Commands, center: Vec2) {
    commands
        .spawn((
            StickBase,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(center.x - STICK_RADIUS),
                top: Val::Px(center.y - STICK_RADIUS),
                width: Val::Px(STICK_RADIUS * 2.0),
                height: Val::Px(STICK_RADIUS * 2.0),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15)),
        ))
        .with_children(|parent| {
            parent.spawn((
                StickKnob,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(STICK_RADIUS - KNOB_RADIUS),
                    top: Val::Px(STICK_RADIUS - KNOB_RADIUS),
                    width: Val::Px(KNOB_RADIUS * 2.0),
                    height: Val::Px(KNOB_RADIUS * 2.0),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
            ));
        });
}

fn despawn_stick(commands: &mut Commands, stick_entities: &Query<Entity, With<StickBase>>) {
    for entity in stick_entities {
        commands.entity(entity).despawn();
    }
}

fn clear_stick(
    mut commands: Commands,
    mut active: ResMut<ActiveStick>,
    mut move_input: ResMut<MoveInput>,
    stick_entities: Query<Entity, With<StickBase>>,
) {
    despawn_stick(&mut commands, &stick_entities);
    active.0 = None;
    move_input.0 = Vec2::ZERO;
}

/// スペースキー / タッチ / マウスクリックのいずれかが押された瞬間に true を返す。
/// タイトル・ウェーブ間・リザルト画面の「タップで進む」判定に使う
pub fn confirm_just_pressed(
    keys: &ButtonInput<KeyCode>,
    touches: &Touches,
    mouse: &ButtonInput<MouseButton>,
) -> bool {
    keys.just_pressed(KeyCode::Space) || touches.any_just_pressed() || mouse.just_pressed(MouseButton::Left)
}
