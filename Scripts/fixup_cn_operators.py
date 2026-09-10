import json
import urllib.request
import os

def check_applicable(desc):
    if not desc: return False
    d = desc.lower()
    return any(k in d for k in ['ally', 'allies', 'team', 'friendly units', 'other operators', 'all supporters', 'all snipers', 'all casters', 'all medics', 'all vanguards', 'all guards', 'all defenders', 'all specialists', 'all operators'])

def download_json(url):
    print(f"Downloading {url}...")
    req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
    with urllib.request.urlopen(req) as res:
        return json.loads(res.read())

def main():
    print("Loading Automated_Operators.json...")
    with open("Automated_Operators.json", "r", encoding="utf-8") as f:
        data = json.load(f)
        
    char_table_cn = download_json("https://raw.githubusercontent.com/Kengxxiao/ArknightsGameData/master/zh_CN/gamedata/excel/character_table.json")
    skill_table_cn = download_json("https://raw.githubusercontent.com/Kengxxiao/ArknightsGameData/master/zh_CN/gamedata/excel/skill_table.json")
    uniequip_table_cn = download_json("https://raw.githubusercontent.com/Kengxxiao/ArknightsGameData/master/zh_CN/gamedata/excel/uniequip_table.json")
    battle_equip_table_cn = download_json("https://raw.githubusercontent.com/Kengxxiao/ArknightsGameData/master/zh_CN/gamedata/excel/battle_equip_table.json")
    
    fixed_count = 0
    for op in data["operators"]:
        if op.get("skills"):
            continue # already has skills
            
        char_id = op["char_id"]
        char_data = char_table_cn.get(char_id)
        if not char_data:
            print(f"Still missing CN data for {char_id}")
            continue
            
        fixed_count += 1
        
        # Base Stats
        phases = char_data.get("phases", [])
        if phases:
            keyframes = phases[-1].get("attributesKeyFrames", [])
            if keyframes:
                stats = keyframes[-1].get("data", {})
                op["base_stats"] = {
                    "hp": stats.get("maxHp", 0),
                    "atk": stats.get("atk", 0),
                    "def": stats.get("def", 0),
                    "res": stats.get("magicResistance", 0),
                    "base_interval": stats.get("baseAttackTime", 1),
                    "dp_cost": stats.get("cost", 0),
                    "redeployment_time": stats.get("respawnTime", 0),
                    "block_count": stats.get("blockCnt", 0)
                }

        # Parse Talents
        talents = []
        for tg in char_data.get("talents") or []:
            cands = tg.get("candidates")
            if not cands: continue
            cand = cands[-1] # max phase
            buffs = []
            for bb in cand.get("blackboard") or []:
                buffs.append({
                    "stat": bb["key"],
                    "type": "blackboard",
                    "value": bb.get("value", 0)
                })
            talents.append({
                "name": cand.get("name", ""),
                "description": cand.get("description", ""),
                "buffs": buffs,
                "applicable_to_others": check_applicable(cand.get("description", ""))
            })
        op["talents"] = talents
        
        # Parse Skills
        skills = []
        for sk in char_data.get("skills") or []:
            skill_id = sk.get("skillId")
            if not skill_id or skill_id not in skill_table_cn:
                continue
                
            skill_data = skill_table_cn[skill_id]
            levels = skill_data.get("levels", [])
            if not levels:
                continue
                
            # M3 (or max level)
            lvl = levels[-1]
            sp_data = lvl.get("spData", {})
            buffs = []
            for bb in lvl.get("blackboard") or []:
                buffs.append({
                    "stat": bb["key"],
                    "type": "blackboard",
                    "value": bb.get("value", 0)
                })
                
            skills.append({
                "name": lvl.get("name", ""),
                "sp_cost": sp_data.get("spCost", 0),
                "initial_sp": sp_data.get("initSp", 0),
                "duration": lvl.get("duration", 0),
                "sp_type": sp_data.get("spType", ""),
                "buffs": buffs,
                "description": lvl.get("description", ""),
                "applicable_to_others": check_applicable(lvl.get("description", ""))
            })
        op["skills"] = skills
        
        # Parse Modules
        modules = []
        char_equips = uniequip_table_cn.get("charEquip", {}).get(char_id, [])
        for eq_id in char_equips:
            eq_data = battle_equip_table_cn.get(eq_id)
            if not eq_data: continue
            
            # Use level 3
            phases = eq_data.get("phases", [])
            if not phases: continue
            lvl3 = phases[-1]
            
            buffs = []
            for bb in lvl3.get("attributeBlackboard") or []:
                stat_map = {"max_hp": "hp", "atk": "atk", "def": "def", "magic_resistance": "res"}
                stat_name = stat_map.get(bb["key"], bb["key"])
                buffs.append({
                    "stat": stat_name,
                    "type": "flat_base",
                    "value": bb.get("value", 0)
                })
                
            for part in lvl3.get("parts") or []:
                override_talent = part.get("addOrOverrideTalentDataBundle", {})
                if override_talent:
                    cands = override_talent.get("candidates")
                    if cands:
                        for bb in cands[-1].get("blackboard") or []:
                            buffs.append({
                                "stat": bb["key"],
                                "type": "blackboard",
                                "value": bb.get("value", 0)
                            })
                            
            modules.append({
                "name": eq_id,
                "level": 3,
                "buffs": buffs,
                "is_true_aoe": False,
                "applicable_to_others": False
            })
        op["modules"] = modules

    with open("Automated_Operators.json", "w", encoding="utf-8") as f:
        json.dump(data, f, indent=4)
        
    print(f"Fixed {fixed_count} CN operators!")

if __name__ == "__main__":
    main()
