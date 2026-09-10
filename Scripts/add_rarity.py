import json
import urllib.request
import re

def download_json(url):
    req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
    with urllib.request.urlopen(req) as res:
        return json.loads(res.read())

def main():
    print("Loading data...")
    char_table_en = download_json("https://raw.githubusercontent.com/Aceship/AN-EN-Tags/master/json/gamedata/en_US/gamedata/excel/character_table.json")
    char_table_cn = download_json("https://raw.githubusercontent.com/Kengxxiao/ArknightsGameData/master/zh_CN/gamedata/excel/character_table.json")
    uniequip_table = download_json("https://raw.githubusercontent.com/Aceship/AN-EN-Tags/master/json/gamedata/en_US/gamedata/excel/uniequip_table.json")
    uniequip_table_cn = download_json("https://raw.githubusercontent.com/Kengxxiao/ArknightsGameData/master/zh_CN/gamedata/excel/uniequip_table.json")
    
    subprof_dict = uniequip_table.get("subProfDict", {})
    subprof_dict_cn = uniequip_table_cn.get("subProfDict", {})
    
    with open("Automated_Operators.json", "r", encoding="utf-8") as f:
        data = json.load(f)
        
    for op in data["operators"]:
        char_id = op["char_id"]
        char_data = char_table_en.get(char_id) or char_table_cn.get(char_id)
        if not char_data:
            continue
            
        # Parse rarity (e.g. "TIER_6" -> 6)
        rarity_str = char_data.get("rarity", "")
        match = re.search(r'\d+', rarity_str)
        if match:
            op["rarity"] = int(match.group())
            
        # Parse profession / subclass
        op["profession"] = char_data.get("profession", "")
        sub_id = char_data.get("subProfessionId", "")
        op["sub_profession_id"] = sub_id
        
        # Get actual name
        if sub_id in subprof_dict:
            op["subclass_name"] = subprof_dict[sub_id].get("subProfessionName", sub_id)
        elif sub_id in subprof_dict_cn:
            op["subclass_name"] = subprof_dict_cn[sub_id].get("subProfessionName", sub_id)
        else:
            op["subclass_name"] = sub_id

    with open("Automated_Operators.json", "w", encoding="utf-8") as f:
        json.dump(data, f, indent=4)
        
    print("Added rarity and classes!")

if __name__ == "__main__":
    main()
