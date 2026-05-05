#!/usr/bin/env python3
# tune_params.py - パラメータ自動調整スクリプト

import json
import subprocess
import os
import sys
from dataclasses import dataclass, asdict
from typing import List, Dict, Any
import time

@dataclass
class ParamConfig:
    """パラメータ設定"""
    name: str
    min_value: int
    max_value: int
    step: int
    
    def get_values(self):
        """テストする値のリストを生成"""
        return list(range(self.min_value, self.max_value + 1, self.step))

@dataclass
class TestResult:
    """テスト結果"""
    params: Dict[str, Any]
    wins: int
    losses: int
    total_games: int
    
    @property
    def win_rate(self):
        return self.wins / self.total_games if self.total_games > 0 else 0.0
    
    def __str__(self):
        return f"Win rate: {self.win_rate:.1%} ({self.wins}W-{self.losses}L)"

class ParamTuner:
    """パラメータチューナー"""
    
    def __init__(self, base_params_file="params_default.json"):
        # ベースパラメータを読み込み
        if os.path.exists(base_params_file):
            with open(base_params_file, 'r') as f:
                self.base_params = json.load(f)
            print(f"Loaded base parameters from {base_params_file}")
        else:
            # デフォルトパラメータを作成
            print(f"{base_params_file} not found, creating default parameters...")
            self.base_params = self.create_default_params()
            # 保存
            with open(base_params_file, 'w') as f:
                json.dump(self.base_params, f, indent=2)
            print(f"Created {base_params_file}")
        
        self.results: List[TestResult] = []
    
    def create_default_params(self) -> Dict[str, Any]:
        """デフォルトパラメータを作成"""
        return {
            "hand_pawn": 100,
            "hand_lance": 350,
            "hand_knight": 450,
            "hand_silver": 550,
            "hand_gold": 600,
            "hand_bishop": 950,
            "hand_rook": 1100,
            "rook_proximity_weight": 4,
            "bishop_proximity_weight": 4,
            "gold_proximity_weight": 3,
            "enemy_zone_gold": 15,
            "enemy_zone_horse": 20,
            "enemy_zone_dragon": 20,
            "enemy_zone_rook": 25,
            "enemy_zone_bishop": 25,
            "enemy_zone_pawn": 10,
            "center_rook": 10,
            "center_bishop": 8,
            "center_gold": 5,
            "tempo_value": 20,
            "king_safety_gold": 15,
            "king_safety_silver": 10,
            "king_safety_propawn": 12,
            "king_safety_pawn": 5,
            "king_edge_bonus": 15,
            "king_center_penalty": -15
        }
    
    def create_params_file(self, params: Dict[str, Any], filename="params.json"):
        """パラメータファイルを作成"""
        # ベースパラメータをコピー
        new_params = self.base_params.copy()
        # 指定されたパラメータで上書き
        new_params.update(params)
        
        # プロジェクトルートに保存
        with open(filename, 'w') as f:
            json.dump(new_params, f, indent=2)
        
        # target/release/にもコピー（エンジンの実行ディレクトリ）
        release_dir = os.path.join("target", "release")
        if os.path.exists(release_dir):
            release_params = os.path.join(release_dir, filename)
            with open(release_params, 'w') as f:
                json.dump(new_params, f, indent=2)
            print(f"Params also saved to {release_params}")
    
    def compile_engine(self):
        """エンジンをコンパイル"""
        print("Compiling engine...")
        result = subprocess.run(
            ['cargo', 'build', '--release', '--bin', 'engine'],
            capture_output=True,
            text=True
        )
        
        if result.returncode != 0:
            print("Compilation failed!")
            print(result.stderr)
            return False
        
        print("Compilation successful!")
        return True
    
    def test_params(self, params: Dict[str, Any], num_games: int = 10) -> TestResult:
        """パラメータをテスト"""
        print(f"\nTesting params: {params}")
        
        # パラメータファイルを作成
        self.create_params_file(params)
        
        # エンジンをコンパイル
        if not self.compile_engine():
            return TestResult(params, 0, 0, 0)
        
        # 対戦結果を入力
        print(f"\nPlease play {num_games} games vs LesserKai using the compiled engine")
        print("Engine path: target/release/engine")
        print("\nWhen finished, enter the results:")
        
        while True:
            try:
                wins = int(input(f"Number of wins (out of {num_games}): "))
                if 0 <= wins <= num_games:
                    break
                print(f"Please enter a number between 0 and {num_games}")
            except ValueError:
                print("Please enter a valid number")
        
        losses = num_games - wins
        result = TestResult(params, wins, losses, num_games)
        
        print(result)
        self.results.append(result)
        
        return result
    
    def grid_search_single_param(self, param_name: str, config: ParamConfig, 
                                  num_games: int = 10, resume: bool = True):
        """単一パラメータのグリッドサーチ"""
        print(f"\n{'='*60}")
        print(f"Grid Search: {param_name}")
        print(f"Range: {config.min_value} to {config.max_value}, step {config.step}")
        print(f"Games per setting: {num_games}")
        print(f"{'='*60}")
        
        values = config.get_values()
        results = []
        
        # 既存の結果を読み込み（resume=Trueの場合）
        resume_file = f"results_{param_name}_progress.json"
        tested_values = set()
        
        if resume and os.path.exists(resume_file):
            print(f"\nFound previous progress file: {resume_file}")
            try:
                with open(resume_file, 'r') as f:
                    previous_results = json.load(f)
                
                for item in previous_results:
                    value = item['value']
                    result = TestResult(
                        params={param_name: value},
                        wins=item['wins'],
                        losses=item['losses'],
                        total_games=item['total_games']
                    )
                    results.append((value, result))
                    tested_values.add(value)
                    print(f"  Loaded: {param_name}={value} - {result}")
                
                print(f"\nResuming from previous progress ({len(tested_values)} values already tested)")
            except Exception as e:
                print(f"Error loading progress file: {e}")
                print("Starting fresh...")
        
        for value in values:
            # 既にテスト済みならスキップ
            if value in tested_values:
                print(f"\nSkipping {param_name}={value} (already tested)")
                continue
            
            params = {param_name: value}
            result = self.test_params(params, num_games)
            results.append((value, result))
            
            # 進捗を保存（途中で中断しても大丈夫）
            progress_data = []
            for v, r in results:
                progress_data.append({
                    'value': v,
                    'wins': r.wins,
                    'losses': r.losses,
                    'total_games': r.total_games,
                    'win_rate': r.win_rate
                })
            
            with open(resume_file, 'w') as f:
                json.dump(progress_data, f, indent=2)
            print(f"Progress saved to {resume_file}")
        
        # 結果表示
        print(f"\n{'='*60}")
        print(f"Results for {param_name}:")
        print(f"{'='*60}")
        
        results.sort(key=lambda x: x[1].win_rate, reverse=True)
        
        for value, result in results:
            print(f"{param_name}={value:4d}: {result}")
        
        best_value, best_result = results[0]
        print(f"\nBest: {param_name}={best_value} with {best_result}")
        
        # 最終結果を保存（progress_dataを再生成）
        progress_data = []
        for v, r in results:
            progress_data.append({
                'value': v,
                'wins': r.wins,
                'losses': r.losses,
                'total_games': r.total_games,
                'win_rate': r.win_rate
            })
        
        final_results_file = f"results_{param_name}_final.json"
        with open(final_results_file, 'w') as f:
            json.dump(progress_data, f, indent=2)
        print(f"Final results saved to {final_results_file}")
        
        return best_value, best_result
    
    def save_results(self, filename: str):
        """結果を保存"""
        data = []
        for result in self.results:
            data.append({
                'params': result.params,
                'wins': result.wins,
                'losses': result.losses,
                'total_games': result.total_games,
                'win_rate': result.win_rate
            })
        
        with open(filename, 'w') as f:
            json.dump(data, f, indent=2)
        
        print(f"Results saved to {filename}")
    
    def load_results(self, filename: str):
        """結果を読み込み"""
        with open(filename, 'r') as f:
            data = json.load(f)
        
        self.results = []
        for item in data:
            result = TestResult(
                params=item['params'],
                wins=item['wins'],
                losses=item['losses'],
                total_games=item['total_games']
            )
            self.results.append(result)
        
        print(f"Loaded {len(self.results)} results from {filename}")

def main():
    """メイン処理"""
    
    # チューナーを初期化
    tuner = ParamTuner()
    
    # 調整するパラメータを定義
    params_to_tune = {
        'hand_rook': ParamConfig('hand_rook', 1000, 1200, 50),
        'hand_bishop': ParamConfig('hand_bishop', 900, 1000, 50),
        'tempo_value': ParamConfig('tempo_value', 15, 25, 5),
        'rook_proximity_weight': ParamConfig('rook_proximity_weight', 3, 5, 1),
    }
    
    print("="*60)
    print("TMOQ Parameter Tuning System")
    print("="*60)
    print("\nParameters to tune:")
    for name, config in params_to_tune.items():
        values = config.get_values()
        print(f"  {name}: {values}")
    
    print("\nEach parameter will be tested with 10 games vs LesserKai")
    print(f"Total games required: {sum(len(c.get_values()) for c in params_to_tune.values()) * 10}")
    
    print("\n" + "="*60)
    print("RESUME FEATURE:")
    print("  Progress is saved after each test.")
    print("  If interrupted, re-run this script to resume.")
    print("  Progress files: results_<param>_progress.json")
    print("="*60)
    
    input("\nPress Enter to start...")
    
    # 各パラメータを順番に調整
    best_params = {}
    
    for param_name, config in params_to_tune.items():
        best_value, best_result = tuner.grid_search_single_param(
            param_name, 
            config, 
            num_games=10
        )
        best_params[param_name] = best_value
        
        # ベストパラメータを更新
        tuner.base_params[param_name] = best_value
    
    # 最終結果
    print("\n" + "="*60)
    print("FINAL RESULTS")
    print("="*60)
    print("\nBest parameters found:")
    for param_name, value in best_params.items():
        print(f"  {param_name}: {value}")
    
    # ベストパラメータを保存
    tuner.create_params_file(best_params, "params_best.json")
    print("\nBest parameters saved to params_best.json")
    
    # 全結果を保存
    tuner.save_results("results_all.json")
    
    print("\nTuning complete!")

if __name__ == '__main__':
    main()
