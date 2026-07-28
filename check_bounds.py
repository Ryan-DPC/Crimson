from PIL import Image

def find_visible_bounds(image_path, threshold):
    img = Image.open(image_path)
    img = img.convert("RGBA")
    width, height = img.size
    
    min_x = width
    min_y = height
    max_x = 0
    max_y = 0
    
    found = False
    for y in range(height):
        for x in range(width):
            _, _, _, a = img.getpixel((x, y))
            if a > threshold:
                found = True
                if x < min_x: min_x = x
                if x > max_x: max_x = x
                if y < min_y: min_y = y
                if y > max_y: max_y = y
                
    if found:
        print(f"Threshold {threshold} - Bounds: ({min_x}, {min_y}) to ({max_x}, {max_y}), Size: {max_x - min_x}x{max_y - min_y}")
    else:
        print(f"Threshold {threshold} - Empty")

path = r"f:\CrimsonProject\crimson\src\assets\logos\logo_red_transparent.png"
find_visible_bounds(path, 10)
find_visible_bounds(path, 50)
find_visible_bounds(path, 128)
find_visible_bounds(path, 200)
