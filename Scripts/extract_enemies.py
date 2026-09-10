import json
import urllib.request
import os

def download_json(url):
    print(f"Downloading {url}...")
    req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
    with urllib.request.urlopen(req) as res:
        return json.loads(res.read())

def main():
    db_url = 'https://raw.githubusercontent.com/Aceship/AN-EN-Tags/master/json/gamedata/zh_CN/gamedata/levels/enemydata/enemy_database.json'
    hb_url = 'https://raw.githubusercontent.com/Aceship/AN-EN-Tags/master/json/gamedata/zh_CN/gamedata/excel/enemy_handbook_table.json'
    
    try:
        db = download_json(db_url)
        hb = download_json(hb_url)
    except Exception as e:
        print("Failed to download data:", e)
        return

    hb_data = hb.get("enemyData", {})
    
    boss_keywords = [
        "特蕾西娅", "特雷西斯", "塔露拉", "曼弗雷德", "变形者", "爱国者", "霜星", "梅菲斯特",
        "浮士德", "弑君者", "泥岩", "杰斯顿", "黑蛇", "大亚当", "大鲍勃", "杜卡雷", "安多恩",
        "克里斯滕", "萨卢佐", "孽茨雷", "食腐者", "蒸汽骑士", "皇帝的利刃", "最后的骑士",
        "拉塔托", "卡米洛", "奎隆", "多索雷斯", "庞贝", "锏", "黑冠", "伊斯"
    ]
    
    enemies = []
    
    for enemy in db.get("enemies", []):
        key = enemy.get("Key")
        levels = enemy.get("Value", [])
        if not levels: continue
        
        base_data = levels[0].get("enemyData", {})
        attrs = base_data.get("attributes", {})
        
        def get_val(attr, default=0.0):
            if attr in attrs and attrs[attr].get("m_defined"):
                return attrs[attr].get("m_value", default)
            return default

        def is_true(attr):
            return get_val(attr, 0.0) > 0.5
            
        hp = get_val("maxHp")
        atk = get_val("atk")
        defn = get_val("def")
        res = get_val("magicResistance")
        weight = get_val("massLevel", 1.0)
        move_speed = get_val("moveSpeed", 1.0)
        attack_interval = get_val("baseAttackTime", 2.5)

        hit_phys = 1.0 if not attrs.get("damageHitratePhysical", {}).get("m_defined") else get_val("damageHitratePhysical", 1.0)
        hit_arts = 1.0 if not attrs.get("damageHitrateMagical", {}).get("m_defined") else get_val("damageHitrateMagical", 1.0)

        hb_info = hb_data.get(key, {})
        name = hb_info.get("name") or base_data.get("name", {}).get("m_value", "Unknown")
        
        # Skills and talents
        skills = base_data.get("skills") or []
        skill_names = [s.get("prefabKey") for s in skills if isinstance(s, dict) and s.get("prefabKey")]
        talents = base_data.get("talentBlackboard") or []
        talent_str = str(talents).lower()
        
        has_revive = any(x in talent_str for x in ['revive', 'reborn', 'life', 'recover']) or any('reborn' in s.lower() for s in skill_names)
        has_invuln = any(x in talent_str for x in ['invulnerable', 'shield', 'damage_res', 'immune']) or any('shield' in s.lower() for s in skill_names)
        has_phase = any(x in talent_str for x in ['phase', 'halfhp', 'transform', 'switch']) or any('phase' in s.lower() or 'switch' in s.lower() for s in skill_names)

        # Classification
        tier = hb_info.get("enemyLevel")
        
        # 1. Suffix stripping parent check
        if not tier:
            for sfx in ['_2', '_3', '_4', '_b', '_s', '_ex', '_final', '_p2', '_p3', '_c', '_d', '_h', '_boss']:
                if key.endswith(sfx):
                    parent = key[:-len(sfx)]
                    if parent in hb_data:
                        tier = hb_data[parent].get("enemyLevel")
                        break

        # 2. 4-digit enemy_15xx are standard Arknights bosses
        key_parts = key.split('_')
        is_15xx = len(key_parts) >= 2 and len(key_parts[1]) == 4 and key_parts[1].startswith('15')
        if not tier and is_15xx:
            if hp >= 5000 or atk >= 300:
                tier = "BOSS"

        # 3. Known Boss name keywords
        if not tier:
            if any(kw in name for kw in boss_keywords):
                tier = "BOSS"

        # 4. Boss stats & mechanics heuristics
        if not tier:
            if (hp >= 40000 and (defn >= 600 or res >= 40)) or (hp >= 25000 and has_revive and defn >= 500):
                tier = "BOSS"
            elif hp >= 18000 and (defn >= 500 or atk >= 700):
                tier = "ELITE"
            else:
                tier = "NORMAL"

        # Fallback for known bosses with 0 base ATK who deal damage via skills/auras
        if key in ['enemy_1276_telex'] and atk < 100:
            atk = 1800.0
        elif key in ['enemy_1115_embald'] and atk < 100:
            atk = 1200.0

        if tier not in ["BOSS", "ELITE", "NORMAL"]:
            tier = "NORMAL"

        e_dict = {
            "id": key,
            "name": name,
            "tier": tier,
            "tier_type": tier,
            "hp": hp,
            "atk": atk,
            "def": defn,
            "res": res,
            "weight": weight,
            "move_speed": move_speed,
            "attack_interval": attack_interval,
            "dodge_phys": max(0.0, 1.0 - hit_phys),
            "dodge_arts": max(0.0, 1.0 - hit_arts),
            "immune_stun": is_true("stunImmune"),
            "immune_silence": is_true("silenceImmune"),
            "immune_sleep": is_true("sleepImmune"),
            "immune_freeze": is_true("frozenImmune"),
            "immune_levitate": is_true("levitateImmune"),
            "skill_count": len(skills),
            "skills": skill_names,
            "has_revive": has_revive,
            "has_shield": has_invuln,
            "has_phase": has_phase
        }
        enemies.append(e_dict)
        
    out_dir = "data"
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "Automated_Enemies.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(enemies, f, indent=2, ensure_ascii=False)
        
    print(f"Exported {len(enemies)} enemies to {out_path}")

if __name__ == "__main__":
    main()
