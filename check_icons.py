from PIL import Image
import glob

def check_icon(path):
    try:
        img = Image.open(path)
        img = img.convert("RGBA")
        width, height = img.size
        
        min_x = width
        min_y = height
        max_x = 0
        max_y = 0
        
        found = False
        for y in range(height):
            for x in range(width):
                r, g, b, a = img.getpixel((x, y))
                if a > 10:
                    found = True
                    if x < min_x: min_x = x
                    if x > max_x: max_x = x
                    if y < min_y: min_y = y
                    if y > max_y: max_y = y
                    
        if found:
            print(f"File {path}: Size {width}x{height}, Content Bounds: ({min_x}, {min_y}) to ({max_x}, {max_y}), Content Size: {max_x - min_x}x{max_y - min_y}")
        else:
            print(f"File {path}: Empty")
    except Exception as e:
        pass

check_icon(r"f:\CrimsonProject\crimson\src-tauri\icons\128x128.png")
check_icon(r"f:\CrimsonProject\crimson\src-tauri\icons\32x32.png")
