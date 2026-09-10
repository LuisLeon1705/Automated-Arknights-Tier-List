import json
import os
import urllib.request
import urllib.error
import time

MYRTLE_DIR = "myrtle-main"
OPERATOR_FORMULAS = os.path.join(MYRTLE_DIR, "backend/src/dps/config/operator_formulas.json")
HEAL_FORMULAS = os.path.join(MYRTLE_DIR, "backend/src/dps/config/heal_formulas.json")
OUTPUT_FILE = "Automated_Operators.json"
IMG_DIR = "static/images"

def download_file(url):
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            return response.read()
    except Exception as e:
        print(f"Failed to download {url}: {e}")
        return None

def extract_operators():
    automated_list = []
    
    print("Downloading character_table.json...")
    char_table_data = download_file("https://raw.githubusercontent.com/Aceship/AN-EN-Tags/master/json/gamedata/en_US/gamedata/excel/character_table.json")
    if not char_table_data:
        print("Could not download character table. Aborting.")
        return
        
    char_table = json.loads(char_table_data)
    
    os.makedirs(IMG_DIR, exist_ok=True)
    
    all_formulas = {}
    if os.path.exists(OPERATOR_FORMULAS):
        with open(OPERATOR_FORMULAS, "r", encoding="utf-8") as f:
            all_formulas.update(json.load(f))
    if os.path.exists(HEAL_FORMULAS):
        with open(HEAL_FORMULAS, "r", encoding="utf-8") as f:
            all_formulas.update(json.load(f))
            
    print(f"Found {len(all_formulas)} formulas. Processing...")
    
    for i, (char_id, op_info) in enumerate(all_formulas.items()):
        if i % 10 == 0:
            print(f"Processing {i}/{len(all_formulas)}...")
            
        base_stats = {}
        if char_id in char_table:
            phases = char_table[char_id].get("phases", [])
            if phases:
                last_phase = phases[-1]
                keyframes = last_phase.get("attributesKeyFrames", [])
                if keyframes:
                    stats = keyframes[-1].get("data", {})
                    base_stats = {
                        "hp": stats.get("maxHp", 0),
                        "atk": stats.get("atk", 0),
                        "def": stats.get("def", 0),
                        "res": stats.get("magicResistance", 0),
                        "base_interval": stats.get("baseAttackTime", 1),
                        "dp_cost": stats.get("cost", 0),
                        "redeployment_time": stats.get("respawnTime", 0),
                        "block_count": stats.get("blockCnt", 0)
                    }
        
        photo_filename = f"{char_id}.png"
        photo_path = os.path.join(IMG_DIR, photo_filename)
        
        # Download image if it doesn't exist
        if not os.path.exists(photo_path):
            img_url = f"https://raw.githubusercontent.com/Aceship/Arknight-Images/main/avatars/{char_id}.png"
            img_data = download_file(img_url)
            if not img_data:
                # Try E2 avatar as fallback
                img_url_2 = f"https://raw.githubusercontent.com/Aceship/Arknight-Images/main/avatars/{char_id}_2.png"
                img_data = download_file(img_url_2)
            if img_data:
                with open(photo_path, "wb") as f:
                    f.write(img_data)
                    
        op_type = "healer" if char_id in open(HEAL_FORMULAS, "r").read() else "damage"
        
        automated_list.append({
            "operator_id": i + 1000,
            "char_id": char_id,
            "name": op_info.get("name", char_id),
            "class_name": op_info.get("class_name", ""),
            "available_skills": op_info.get("available_skills", []),
            "available_modules": op_info.get("available_modules", []),
            "type": op_type,
            "conditionals": op_info.get("conditionals", []),
            "base_stats": base_stats,
            "photo_path": photo_filename
        })

    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        json.dump({"operators": automated_list}, f, indent=4)
        
    print(f"Extracted {len(automated_list)} operators with stats and images into {OUTPUT_FILE}")

if __name__ == "__main__":
    extract_operators()
