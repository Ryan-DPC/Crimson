from PIL import Image
import os
import glob

def make_icons(source_path, target_dir):
    # Open perfectly cropped image
    img = Image.open(source_path)
    img = img.convert("RGBA")
    
    # We want exactly 0 extra padding beyond what's already there
    
    # List of required sizes based on what Tauri generated
    png_sizes = [
        32, 64, 128, 256, 512,  # standard powers of 2
    ]
    
    # generate .ico which contains multiple sizes
    ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    
    # First, let's create a perfectly square version with 0 padding
    # Wait, the source path should be cropped EXACTLY to bounds, then made square
    # without adding ANY padding
    
    width, height = img.size
    min_x = width
    min_y = height
    max_x = 0
    max_y = 0
    found = False
    for y in range(height):
        for x in range(width):
            r, g, b, a = img.getpixel((x, y))
            if a > 200:
                found = True
                if x < min_x: min_x = x
                if x > max_x: max_x = x
                if y < min_y: min_y = y
                if y > max_y: max_y = y
                
    if not found:
        print("Empty image")
        return
        
    cropped = img.crop((min_x, min_y, max_x, max_y))
    target_size = max(max_x - min_x, max_y - min_y)
    
    # Square with 0 padding
    square_img = Image.new("RGBA", (target_size, target_size), (0, 0, 0, 0))
    paste_x = (target_size - (max_x - min_x)) // 2
    paste_y = (target_size - (max_y - min_y)) // 2
    square_img.paste(cropped, (paste_x, paste_y))
    
    # Save .ico
    square_img.save(os.path.join(target_dir, "icon.ico"), format="ICO", sizes=ico_sizes)
    
    # Save .pngs
    # Tauri creates specific names
    names = [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512)
    ]
    
    for name, size in names:
        resized = square_img.resize((size, size), Image.Resampling.LANCZOS)
        resized.save(os.path.join(target_dir, name))
        
    # Also overwrite Square30x30Logo.png etc.
    windows_icons = [
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
    for name, size in windows_icons:
        resized = square_img.resize((size, size), Image.Resampling.LANCZOS)
        resized.save(os.path.join(target_dir, name))
        
    print("Replaced all icons with 0 padding versions!")

make_icons(r"f:\CrimsonProject\crimson\src\assets\logos\logo_mark_only.png", r"f:\CrimsonProject\crimson\src-tauri\icons")
