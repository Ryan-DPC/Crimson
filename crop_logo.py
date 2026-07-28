from PIL import Image

def crop_transparent(image_path, output_path):
    img = Image.open(image_path)
    img = img.convert("RGBA")
    bbox = img.getbbox()
    if bbox:
        print(f"Cropping from {img.size} to {bbox}")
        cropped = img.crop(bbox)
        
        # Add a tiny bit of padding (like 5%) so it doesn't touch the absolute edge
        width, height = cropped.size
        pad_x = int(width * 0.05)
        pad_y = int(height * 0.05)
        
        padded = Image.new("RGBA", (width + 2*pad_x, height + 2*pad_y), (0, 0, 0, 0))
        padded.paste(cropped, (pad_x, pad_y))
        
        padded.save(output_path)
        print("Done!")
    else:
        print("Image is entirely transparent.")

crop_transparent(
    r"f:\CrimsonProject\crimson\src\assets\logos\logo_red_transparent.png",
    r"f:\CrimsonProject\crimson\src\assets\logos\logo_red_transparent_cropped.png"
)
