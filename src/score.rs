use bevy::prelude::*;

use crate::GameState;
use crate::enemy::EnemyDied;

/// 現在のランの累計スコア（ワールドに1つだけのグローバルデータ）
#[derive(Resource, Default)]
pub struct Score(pub u32);

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .add_systems(OnEnter(GameState::Playing), reset_score)
            .add_systems(Update, add_kill_score.run_if(in_state(GameState::Playing)));
    }
}

/// ラン開始時にスコアを0に戻す
fn reset_score(mut score: ResMut<Score>) {
    score.0 = 0;
}

/// 敵の死亡メッセージを受け取り、その敵のスコアを加算する
fn add_kill_score(mut score: ResMut<Score>, mut died: MessageReader<EnemyDied>) {
    for event in died.read() {
        score.0 += event.score;
    }
}
