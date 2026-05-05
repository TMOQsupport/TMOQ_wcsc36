#!/usr/bin/env python3
# test_single_param.py - 単一パラメータのクイックテスト

import json
import subprocess
import os

def create_params(params_dict, filename="params.json"):
    """パラメータファイルを作成"""
    # プロジェクトルートに保存
    with open(filename, 'w') as f:
        json.dump(params_dict, f, indent=2)
    print(f"Created {filename}")
    
    # target/release/にもコピー（エンジンの実行ディレクトリ）
    release_dir = os.path.join("target", "release")
    if os.path.exists(release_dir):
        release_params = os.path.join(release_dir, filename)
        with open(release_params, 'w') as f:
            json.dump(params_dict, f, indent=2)
        print(f"Also created {release_params}")
    
    print(f"Parameters: {params_dict}")

def compile_engine():
    """エンジンをコンパイル"""
    print("\nChecking if engine needs compilation...")
    
    engine_path = os.path.join("target", "release", "engine.exe")
    
    # エンジンが存在するかチェック
    if os.path.exists(engine_path):
        # タイムスタンプを表示
        import datetime
        mtime = os.path.getmtime(engine_path)
        timestamp = datetime.datetime.fromtimestamp(mtime)
        print(f"Engine found: {engine_path}")
        print(f"Last compiled: {timestamp}")
        print("\nNote: params.json is loaded at runtime.")
        print("      No recompilation needed for parameter changes!")
        return True
    else:
        print("Engine not found, compiling...")
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

def main():
    # デフォルトパラメータ
    default_params = {
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
    
    print("="*60)
    print("TMOQ Single Parameter Test")
    print("="*60)
    print("\nIMPORTANT:")
    print("  Parameters are loaded from params.json at runtime.")
    print("  No recompilation needed when changing parameters!")
    print("  The same engine.exe is used with different params.json")
    print("="*60)
    
    # デフォルトパラメータを保存
    with open("params_default.json", 'w') as f:
        json.dump(default_params, f, indent=2)
    print("Saved default parameters to params_default.json")
    
    print("\nWhat would you like to test?")
    print("1. Test default parameters")
    print("2. Test custom parameter")
    print("3. Quick test: hand_rook values (1000, 1100, 1200)")
    
    choice = input("\nEnter choice (1-3): ").strip()
    
    if choice == "1":
        # デフォルトパラメータでテスト
        print("\nTesting with DEFAULT parameters...")
        create_params(default_params)
        if compile_engine():
            print("\nEngine compiled with default parameters!")
            print("Test it against LesserKai and record the results.")
    
    elif choice == "2":
        # カスタムパラメータ
        print("\nEnter parameter to test:")
        param_name = input("Parameter name (e.g., hand_rook): ").strip()
        
        if param_name not in default_params:
            print(f"Error: {param_name} not found in parameters")
            return
        
        param_value = int(input(f"New value for {param_name} (default: {default_params[param_name]}): ").strip())
        
        test_params = default_params.copy()
        test_params[param_name] = param_value
        
        print(f"\nTesting with {param_name} = {param_value}")
        create_params(test_params)
        if compile_engine():
            print(f"\nEngine compiled with {param_name}={param_value}!")
            print("Test it against LesserKai.")
    
    elif choice == "3":
        # hand_rookのクイックテスト
        print("\nQuick test: hand_rook values")
        values = [1000, 1100, 1200]
        
        # 既存の結果を読み込み
        tested_values = {}
        if os.path.exists('results_quick.json'):
            print("\nFound previous results in results_quick.json")
            with open('results_quick.json', 'r') as f:
                previous_results = json.load(f)
            
            for r in previous_results:
                tested_values[r['hand_rook']] = r
                print(f"  Already tested: hand_rook={r['hand_rook']} - {r['win_rate']:.1%} ({r['wins']}W-{r['losses']}L)")
            
            resume = input("\nResume from previous results? (y/n): ").strip().lower()
            if resume != 'y':
                print("Starting fresh...")
                tested_values = {}
                # results_quick.jsonをバックアップ
                import shutil
                import datetime
                backup_name = f"results_quick_backup_{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
                shutil.copy('results_quick.json', backup_name)
                print(f"Previous results backed up to {backup_name}")
        
        for value in values:
            # 既にテスト済みならスキップ
            if value in tested_values:
                skip = input(f"\nhand_rook={value} already tested. Skip? (y/n): ").strip().lower()
                if skip == 'y':
                    print(f"Skipping hand_rook={value}")
                    continue
            
            print(f"\n{'='*60}")
            print(f"Testing hand_rook = {value}")
            print(f"{'='*60}")
            
            test_params = default_params.copy()
            test_params['hand_rook'] = value
            
            create_params(test_params)
            if not compile_engine():
                print("Compilation failed, skipping this test")
                continue
            
            print(f"\nEngine ready with hand_rook={value}")
            print("Please test against LesserKai (10 games recommended)")
            
            wins = input(f"Enter wins (out of 10) for hand_rook={value}: ").strip()
            
            try:
                wins = int(wins)
                win_rate = wins / 10
                print(f"Win rate: {win_rate:.1%} ({wins}/10)")
                
                # 結果を保存
                result = {
                    'hand_rook': value,
                    'wins': wins,
                    'losses': 10 - wins,
                    'win_rate': win_rate
                }
                
                # results.jsonを更新
                results = []
                if os.path.exists('results_quick.json'):
                    with open('results_quick.json', 'r') as f:
                        results = json.load(f)
                    # 同じhand_rookの結果があれば削除
                    results = [r for r in results if r['hand_rook'] != value]
                
                results.append(result)
                
                with open('results_quick.json', 'w') as f:
                    json.dump(results, f, indent=2)
                
                print(f"Result saved to results_quick.json")
                
            except ValueError:
                print("Invalid input, skipping result save")
        
        # 結果サマリー
        if os.path.exists('results_quick.json'):
            print("\n" + "="*60)
            print("RESULTS SUMMARY")
            print("="*60)
            
            with open('results_quick.json', 'r') as f:
                results = json.load(f)
            
            results.sort(key=lambda x: x['win_rate'], reverse=True)
            
            for r in results:
                print(f"hand_rook={r['hand_rook']:4d}: {r['win_rate']:.1%} ({r['wins']}W-{r['losses']}L)")
            
            best = results[0]
            print(f"\nBest: hand_rook={best['hand_rook']} with {best['win_rate']:.1%}")
    
    else:
        print("Invalid choice")

if __name__ == '__main__':
    main()
