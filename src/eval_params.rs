// src/eval_params.rs
// 評価関数のパラメータ

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvalParams {
    // 持ち駒の価値
    pub hand_pawn: i32,
    pub hand_lance: i32,
    pub hand_knight: i32,
    pub hand_silver: i32,
    pub hand_gold: i32,
    pub hand_bishop: i32,
    pub hand_rook: i32,
    
    // 位置評価の重み
    pub rook_proximity_weight: i32,
    pub bishop_proximity_weight: i32,
    pub gold_proximity_weight: i32,
    
    // 敵陣ボーナス
    pub enemy_zone_gold: i32,
    pub enemy_zone_horse: i32,
    pub enemy_zone_dragon: i32,
    pub enemy_zone_rook: i32,
    pub enemy_zone_bishop: i32,
    pub enemy_zone_pawn: i32,
    
    // 中央制御ボーナス
    pub center_rook: i32,
    pub center_bishop: i32,
    pub center_gold: i32,
    
    // 手番の価値
    pub tempo_value: i32,
    
    // 玉の安全度
    pub king_safety_gold: i32,
    pub king_safety_silver: i32,
    pub king_safety_propawn: i32,
    pub king_safety_pawn: i32,
    pub king_edge_bonus: i32,
    pub king_center_penalty: i32,
    
    // 評価値のランダムノイズ（0=なし、推奨: 3-5）
    pub eval_noise: i32,
}

impl Default for EvalParams {
    fn default() -> Self {
        EvalParams {
            // 持ち駒（盤上より少し高め）
            hand_pawn: 100,
            hand_lance: 350,
            hand_knight: 450,
            hand_silver: 550,
            hand_gold: 600,
            hand_bishop: 950,
            hand_rook: 1100,
            
            // 位置評価の重み
            rook_proximity_weight: 4,
            bishop_proximity_weight: 4,
            gold_proximity_weight: 3,
            
            // 敵陣ボーナス
            enemy_zone_gold: 15,
            enemy_zone_horse: 20,
            enemy_zone_dragon: 20,
            enemy_zone_rook: 25,
            enemy_zone_bishop: 25,
            enemy_zone_pawn: 10,
            
            // 中央制御
            center_rook: 10,
            center_bishop: 8,
            center_gold: 5,
            
            // 手番の価値
            tempo_value: 20,
            
            // 玉の安全度
            king_safety_gold: 15,
            king_safety_silver: 10,
            king_safety_propawn: 12,
            king_safety_pawn: 5,
            king_edge_bonus: 15,
            king_center_penalty: -15,
            
            // ランダムノイズ（0=なし、3-5推奨）
            eval_noise: 0,  // デフォルトはオフ
        }
    }
}

impl EvalParams {
    // JSONファイルから読み込み
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let params: EvalParams = serde_json::from_str(&content)?;
        Ok(params)
    }
    
    // JSONファイルに保存
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// グローバルパラメータ（シンプルな実装）
use std::sync::OnceLock;

static EVAL_PARAMS: OnceLock<EvalParams> = OnceLock::new();

pub fn get_eval_params() -> &'static EvalParams {
    EVAL_PARAMS.get_or_init(|| {
        // params.jsonがあれば読み込む、なければデフォルト
        match EvalParams::from_file("params.json") {
            Ok(params) => {
                eprintln!("Loaded params from params.json");
                params
            }
            Err(_) => {
                eprintln!("Using default params");
                EvalParams::default()
            }
        }
    })
}

// デフォルトパラメータを保存
pub fn save_default_params() -> Result<(), Box<dyn std::error::Error>> {
    let params = EvalParams::default();
    params.to_file("params_default.json")?;
    println!("Saved default parameters to params_default.json");
    Ok(())
}
