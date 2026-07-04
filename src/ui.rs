use bevy::prelude::*;

use crate::Health;
use crate::player::Player;

/// HP表示テキストのマーカーコンポーネント
#[derive(Component)]
struct HpText;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud)
            .add_systems(Update, update_hp_text);
    }
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        HpText,
        Text::new("HP: -"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn update_hp_text(
    player: Single<&Health, With<Player>>,
    mut text: Single<&mut Text, With<HpText>>,
) {
    text.0 = format!("HP: {:.0} / {:.0}", player.current.max(0.0), player.max);
}
