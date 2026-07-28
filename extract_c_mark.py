from PIL import Image

def find_c_bounds(image_path):
    img = Image.open(image_path)
    img = img.convert("RGBA")
    width, height = img.size
    
    # We want to find the bounds of the red 'C', not the white text.
    # The text is likely white or gray. The C is red.
    # Let's filter by redness! (R > G + 50 and R > B + 50) and a > 100
    
    min_x = width
    min_y = height
    max_x = 0
    max_y = 0
    
    found = False
    for y in range(height):
        for x in range(width):
            r, g, b, a = img.getpixel((x, y))
            if a > 150 and r > g + 50 and r > b + 50:
                found = True
                if x < min_x: min_x = x
                if x > max_x: max_x = x
                if y < min_y: min_y = y
                if y > max_y: max_y = y
                
    if found:
        print(f"Red C Bounds: ({min_x}, {min_y}) to ({max_x}, {max_y}), Size: {max_x - min_x}x{max_y - min_y}")
        
        # Crop exactly to this
        cropped = img.crop((min_x, min_y, max_x, max_y))
        
        # Square with 0 padding
        target_size = max(max_x - min_x, max_y - min_y)
        square_img = Image.new("RGBA", (target_size, target_size), (0, 0, 0, 0))
        paste_x = (target_size - (max_x - min_x)) // 2
        paste_y = (target_size - (max_y - min_y)) // 2
        square_img.paste(cropped, (paste_x, paste_y))
        
        square_img.save(r"f:\CrimsonProject\crimson\src\assets\logos\logo_mark_only.png")
        print("Saved logo_mark_only.png")
    else:
        print("Empty image")

path = r"f:\CrimsonProject\crimson\src\assets\logos\logo_red_transparent.png"
find_c_bounds(path)
