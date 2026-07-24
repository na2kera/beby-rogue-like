use bevy::prelude::*;

use crate::weapon::WeaponType;

/// UIテキストの表示言語（デフォルトは英語）
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Japanese,
}

impl Language {
    pub fn toggle(self) -> Self {
        match self {
            Language::English => Language::Japanese,
            Language::Japanese => Language::English,
        }
    }

    pub fn press_start(self) -> &'static str {
        match self {
            Language::English => "Tap or Press SPACE to Start",
            Language::Japanese => "タップかスペースキーでスタート",
        }
    }

    pub fn move_hint(self) -> &'static str {
        match self {
            Language::English => "Move: Drag / WASD / Arrow Keys",
            Language::Japanese => "移動: ドラッグ / WASD / 矢印キー",
        }
    }

    pub fn lang_hint(self) -> &'static str {
        match self {
            Language::English => "Language: English (Tap or Press L)",
            Language::Japanese => "言語: 日本語（タップかLキーで切替）",
        }
    }

    pub fn wave_cleared(self, number: u32) -> String {
        match self {
            Language::English => format!("Wave {number} Cleared!"),
            Language::Japanese => format!("ウェーブ {number} クリア！"),
        }
    }

    pub fn press_next_wave(self) -> &'static str {
        match self {
            Language::English => "Tap or Press SPACE for next wave",
            Language::Japanese => "タップかスペースキーで次のウェーブへ",
        }
    }

    pub fn victory(self) -> &'static str {
        match self {
            Language::English => "VICTORY!",
            Language::Japanese => "勝利！",
        }
    }

    pub fn game_over(self) -> &'static str {
        match self {
            Language::English => "GAME OVER",
            Language::Japanese => "ゲームオーバー",
        }
    }

    pub fn score(self, points: u32) -> String {
        match self {
            Language::English => format!("Score: {points}"),
            Language::Japanese => format!("スコア: {points}"),
        }
    }

    pub fn reached_wave(self, number: u32) -> String {
        match self {
            Language::English => format!("Reached Wave {number}"),
            Language::Japanese => format!("到達ウェーブ: {number}"),
        }
    }

    pub fn press_return_title(self) -> &'static str {
        match self {
            Language::English => "Tap or Press SPACE to return to title",
            Language::Japanese => "タップかスペースキーでタイトルへ戻る",
        }
    }

    pub fn weapon_name(self, weapon_type: WeaponType) -> &'static str {
        match self {
            Language::English => match weapon_type {
                WeaponType::Sword => "Sword",
                WeaponType::Bow => "Bow",
                WeaponType::Spear => "Spear",
                WeaponType::Aura => "Aura",
                WeaponType::Bomb => "Bomb",
            },
            Language::Japanese => match weapon_type {
                WeaponType::Sword => "剣",
                WeaponType::Bow => "弓",
                WeaponType::Spear => "槍",
                WeaponType::Aura => "オーラ",
                WeaponType::Bomb => "ボム",
            },
        }
    }
}

/// 全テキスト共通で使うフォント（日本語対応の Noto Sans JP）
#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

pub struct LangPlugin;

impl Plugin for LangPlugin {
    fn build(&self, app: &mut App) {
        // フォントはプラグイン構築時に読み込む。
        // 初期ステートの OnEnter（タイトル画面生成）は PreStartup より先に走るため、
        // システムでの読み込みでは間に合わず UiFont 未登録でパニックする
        let font = app
            .world()
            .resource::<AssetServer>()
            .load("fonts/NotoSansJP-Regular.otf");
        app.insert_resource(UiFont(font))
            .insert_resource(Language::English);
    }
}
