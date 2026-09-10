import json

# Manual mapping from subProfessionId to (class_id, branch_id)
mapping = {
    # Caster (1)
    "blastcaster": (1, 101),
    "chain": (1, 102),
    "corecaster": (1, 103),
    "funnel": (1, 104),
    "mystic": (1, 105),
    "phalanx": (1, 106),
    "splashcaster": (1, 107),
    "primocaster": (1, 103), # Primal caster? Let's fallback to core for now

    # Defender (2)
    "artsprotector": (2, 201),
    "duelist": (2, 202),
    "fortress": (2, 203),
    "guardian": (2, 204),
    "unyield": (2, 205),
    "protector": (2, 206),
    "shotprotector": (2, 207),

    # Guard (3)
    "artsfghter": (3, 301),
    "centurion": (3, 302),
    "crusher": (3, 303),
    "fearless": (3, 304),
    "fighter": (3, 305),
    "instructor": (3, 306),
    "librator": (3, 307),
    "lord": (3, 308),
    "musha": (3, 309),
    "reaper": (3, 310),
    "sword": (3, 311),
    "loopshooter": (3, 308), # fallback to lord

    # Medic (4)
    "physician": (4, 401),
    "ringhealer": (4, 402),
    "healer": (4, 403),
    "wandermedic": (4, 404),
    "incantationmedic": (4, 405),
    "chainhealer": (4, 406),

    # Sniper (5)
    "aoesniper": (5, 501),
    "siegesniper": (5, 502),
    "longrange": (5, 503),
    "bombarder": (5, 504),
    "closerange": (5, 505),
    "hunter": (5, 506),
    "fastshot": (5, 507),
    "reaperrange": (5, 508),

    # Specialist (6)
    "stalker": (6, 601),
    "dollkeeper": (6, 602),
    "executor": (6, 603),
    "geek": (6, 604),
    "hookmaster": (6, 605),
    "merchant": (6, 606),
    "pusher": (6, 607),
    "traper": (6, 608),

    # Supporter (7)
    "blessing": (7, 701),
    "craftsman": (7, 702),
    "bard": (7, 703),
    "slower": (7, 704),
    "underminer": (7, 705),
    "ritualist": (7, 706),
    "summoner": (7, 707),

    # Vanguard (8)
    "agent": (8, 801),
    "charger": (8, 802),
    "pioneer": (8, 803),
    "bearer": (8, 804),
    "tactician": (8, 805),
    "hammer": (3, 302),
    "primcaster": (1, 103),
    "soulcaster": (1, 104),
    "primguard": (3, 311),
    "mercenary": (5, 507),
    "skywalker": (5, 507),
    "alchemist": (4, 401),
    "primprotector": (2, 206)
}

def main():
    print("Loading Automated_Operators.json...")
    with open("Automated_Operators.json", "r", encoding="utf-8") as f:
        data = json.load(f)
        
    fixed = 0
    for op in data["operators"]:
        sub_id = op.get("sub_profession_id", "").lower()
        if not sub_id:
            continue
            
        if sub_id in mapping:
            op["class_id"] = mapping[sub_id][0]
            op["branch_id"] = mapping[sub_id][1]
            fixed += 1
        else:
            print(f"Unknown subProfessionId: {sub_id} for {op['name']}")

    with open("Automated_Operators.json", "w", encoding="utf-8") as f:
        json.dump(data, f, indent=4)
        
    print(f"Added class_id and branch_id to {fixed} operators!")

if __name__ == "__main__":
    main()
