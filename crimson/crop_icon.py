from PIL import Image
import sys

def crop_and_square(input_path, output_path):
    img = Image.open(input_path).convert("RGBA")
    
    # Get bounding box of non-transparent pixels
    bbox = img.getbbox()
    if not bbox:
        print("Image is entirely transparent.")
        return
    
    # Crop to bounding box
    cropped = img.crop(bbox)
    
    # Make it a square by padding with transparency
    w, h = cropped.size
    size = max(w, h)
    
    # Create new transparent square image
    new_img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    
    # Paste the cropped image into the center
    x = (size - w) // 2
    y = (size - h) // 2
    new_img.paste(cropped, (x, y))
    
    # Save the result
    new_img.save(output_path)
    print(f"Saved optimized icon to {output_path} (size: {size}x{size})")

if __name__ == "__main__":
    crop_and_square(sys.argv[1], sys.argv[2])
