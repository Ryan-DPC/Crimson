from PIL import Image
import os

def create_perfect_icons():
    # 1. Open clean C mark
    c_mark = Image.open(r"f:\CrimsonProject\crimson\src\assets\logos\logo_mark_only.png").convert("RGBA")
    
    # 2. Get true bounding box
    bbox = c_mark.getbbox()
    cropped = c_mark.crop(bbox)
    w, h = cropped.size
    
    # 3. Create square canvas with 5% margin so edges aren't clipped by Windows taskbar / shortcuts
    max_dim = max(w, h)
    padding = int(max_dim * 0.05)
    canvas_size = max_dim + (padding * 2)
    
    square = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    paste_x = (canvas_size - w) // 2
    paste_y = (canvas_size - h) // 2
    square.paste(cropped, (paste_x, paste_y))
    
    # Save app-icon.png
    app_icon_path = r"f:\CrimsonProject\crimson\app-icon.png"
    square.save(app_icon_path)
    print(f"Generated app-icon.png: {canvas_size}x{canvas_size} (C mark size: {w}x{h})")
    
    # 4. Generate all Tauri icons
    target_dir = r"f:\CrimsonProject\crimson\src-tauri\icons"
    
    names_and_sizes = [
        ("32x32.png", 32),
        ("64x64.png", 64),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
        ("Square30x30Logo.png", 30),
        ("Square44x44Logo.png", 44),
        ("Square71x71Logo.png", 71),
        ("Square89x89Logo.png", 89),
        ("Square107x107Logo.png", 107),
        ("Square142x142Logo.png", 142),
        ("Square150x150Logo.png", 150),
        ("Square284x284Logo.png", 284),
        ("Square310x310Logo.png", 310),
        ("StoreLogo.png", 50)
    ]
    
    for name, sz in names_and_sizes:
        resized = square.resize((sz, sz), Image.Resampling.LANCZOS)
        resized.save(os.path.join(target_dir, name))
        
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    square.save(os.path.join(target_dir, "icon.ico"), format="ICO", sizes=ico_sizes)
    print("Successfully updated icon.ico and all PNG icons in src-tauri/icons!")

if __name__ == "__main__":
    create_perfect_icons()
