from PIL import Image

def find_actual_bounds(image_path):
    img = Image.open(image_path)
    img = img.convert("RGBA")
    
    # Get bounding box of non-transparent pixels
    # We want to ignore pixels with alpha < 10 (almost transparent)
    width, height = img.size
    
    min_x = width
    min_y = height
    max_x = 0
    max_y = 0
    
    found = False
    
    for y in range(height):
        for x in range(width):
            r, g, b, a = img.getpixel((x, y))
            if a > 10:  # If pixel is somewhat visible
                found = True
                if x < min_x: min_x = x
                if x > max_x: max_x = x
                if y < min_y: min_y = y
                if y > max_y: max_y = y
                
    if found:
        print(f"Original size: {img.size}")
        print(f"Actual content bounds: ({min_x}, {min_y}) to ({max_x}, {max_y})")
        print(f"Content size: {max_x - min_x} x {max_y - min_y}")
        
        # Crop exactly to this
        cropped = img.crop((min_x, min_y, max_x, max_y))
        
        # Now we want to place it in a square with MINIMAL padding
        target_size = max(max_x - min_x, max_y - min_y)
        pad = int(target_size * 0.05) # 5% padding
        
        square_size = target_size + 2 * pad
        
        square_img = Image.new("RGBA", (square_size, square_size), (0, 0, 0, 0))
        
        # center it
        paste_x = pad + (target_size - (max_x - min_x)) // 2
        paste_y = pad + (target_size - (max_y - min_y)) // 2
        
        square_img.paste(cropped, (paste_x, paste_y))
        
        square_img.save(r"f:\CrimsonProject\crimson\src\assets\logos\logo_red_transparent_cropped.png")
        print("Saved perfectly cropped square logo.")
    else:
        print("Image is entirely empty/transparent.")

find_actual_bounds(r"f:\CrimsonProject\crimson\src\assets\logos\logo_red_transparent.png")
