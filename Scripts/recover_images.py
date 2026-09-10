import json
import urllib.request
import urllib.error
import os

IMG_DIR = "static/images"

def download_file(url):
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            return response.read()
    except Exception as e:
        return None

def main():
    print("Loading Automated_Operators.json...")
    with open("Automated_Operators.json", "r", encoding="utf-8") as f:
        data = json.load(f)
        
    for op in data["operators"]:
        char_id = op["char_id"]
        photo_path = os.path.join(IMG_DIR, f"{char_id}.png")
        if not os.path.exists(photo_path):
            img_url = f"https://raw.githubusercontent.com/yuanyan3060/ArknightsGameResource/main/avatar/{char_id}.png"
            img_data = download_file(img_url)
            if not img_data:
                img_url_2 = f"https://raw.githubusercontent.com/yuanyan3060/ArknightsGameResource/main/avatar/{char_id}_2.png"
                img_data = download_file(img_url_2)
            if img_data:
                with open(photo_path, "wb") as f:
                    f.write(img_data)
                print(f"Recovered image for {char_id}")
            else:
                print(f"Still no image for {char_id}")
                
if __name__ == "__main__":
    main()
