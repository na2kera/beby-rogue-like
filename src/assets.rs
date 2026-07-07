use bevy::prelude::*;

use crate::weapon::WeaponType;

/// ゲーム中で使うスプライト画像のハンドル置き場。
/// 起動時に一度だけ読み込み、各スポーン処理はここからハンドルを複製して使う
#[derive(Resource)]
pub struct SpriteAssets {
    pub player: Handle<Image>,
    /// 歩行アニメーション（32x32 のコマが横に4つ並んだスプライトシート）
    pub player_walk: Handle<Image>,
    /// player_walk をコマ分割するためのレイアウト
    pub player_walk_layout: Handle<TextureAtlasLayout>,
    pub enemy_normal: Handle<Image>,
    pub enemy_fast: Handle<Image>,
    pub enemy_tank: Handle<Image>,
    pub boss: Handle<Image>,
    pub arrow: Handle<Image>,
    pub spear: Handle<Image>,
    pub bomb: Handle<Image>,
    pub slash: Handle<Image>,
    pub aura: Handle<Image>,
    pub explosion: Handle<Image>,
    pub floor_tile: Handle<Image>,
    pub wall_tile: Handle<Image>,
    pub drop_sword: Handle<Image>,
    pub drop_bow: Handle<Image>,
    pub drop_spear: Handle<Image>,
    pub drop_aura: Handle<Image>,
    pub drop_bomb: Handle<Image>,
    pub drop_heal: Handle<Image>,
}

/// player_walk.png の1コマの大きさとコマ数
pub const PLAYER_WALK_FRAME_SIZE: u32 = 32;
pub const PLAYER_WALK_FRAME_COUNT: usize = 4;

impl SpriteAssets {
    /// 武器ドロップ品のアイコン画像を返す
    pub fn weapon_drop_icon(&self, weapon_type: WeaponType) -> Handle<Image> {
        match weapon_type {
            WeaponType::Sword => self.drop_sword.clone(),
            WeaponType::Bow => self.drop_bow.clone(),
            WeaponType::Spear => self.drop_spear.clone(),
            WeaponType::Aura => self.drop_aura.clone(),
            WeaponType::Bomb => self.drop_bomb.clone(),
        }
    }
}

pub struct SpriteAssetsPlugin;

impl Plugin for SpriteAssetsPlugin {
    fn build(&self, app: &mut App) {
        // UiFont と同じく、Startup システムより前に確実に使えるよう
        // プラグイン構築時に読み込んでおく
        let layout = TextureAtlasLayout::from_grid(
            UVec2::splat(PLAYER_WALK_FRAME_SIZE),
            PLAYER_WALK_FRAME_COUNT as u32,
            1,
            None,
            None,
        );
        let player_walk_layout = app
            .world_mut()
            .resource_mut::<Assets<TextureAtlasLayout>>()
            .add(layout);

        let server = app.world().resource::<AssetServer>();
        let sprites = SpriteAssets {
            player: server.load("sprites/player.png"),
            player_walk: server.load("sprites/player_walk.png"),
            player_walk_layout,
            enemy_normal: server.load("sprites/enemy_normal.png"),
            enemy_fast: server.load("sprites/enemy_fast.png"),
            enemy_tank: server.load("sprites/enemy_tank.png"),
            boss: server.load("sprites/boss.png"),
            arrow: server.load("sprites/arrow.png"),
            spear: server.load("sprites/spear.png"),
            bomb: server.load("sprites/bomb.png"),
            slash: server.load("sprites/slash.png"),
            aura: server.load("sprites/aura.png"),
            explosion: server.load("sprites/explosion.png"),
            floor_tile: server.load("sprites/floor_tile.png"),
            wall_tile: server.load("sprites/wall_tile.png"),
            drop_sword: server.load("sprites/drop_sword.png"),
            drop_bow: server.load("sprites/drop_bow.png"),
            drop_spear: server.load("sprites/drop_spear.png"),
            drop_aura: server.load("sprites/drop_aura.png"),
            drop_bomb: server.load("sprites/drop_bomb.png"),
            drop_heal: server.load("sprites/drop_heal.png"),
        };
        app.insert_resource(sprites);
    }
}
