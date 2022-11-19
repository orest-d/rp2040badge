from PIL import Image, ImageOps
import numpy as np
from glob import glob
#from bytesio import BytesIO

for path in glob("images/*.png"):
    name = path.replace("images/","").replace(".png","")
    print(name)

    img = Image.open(path).convert('RGB')
    img = img.transpose(method=Image.FLIP_TOP_BOTTOM)
    img = img.transpose(method=Image.FLIP_LEFT_RIGHT)
    #img = ImageOps.flip(Image.open(path).convert('RGB'))
    red, green, blue = (np.array(x, np.float)/255.0 for x in img.split())
    print(red.shape, np.min(red), np.max(red))
    shape = red.shape

    R5 = np.array(red*0b11111, np.uint16) << 11
    G6 = np.array(green*0b111111, np.uint16) << 5
    B5 = np.array(blue*0b11111, np.uint16)

    data = (R5 | G6 | B5).flatten().byteswap()
    b = bytes([shape[0],shape[1]]) + data.tobytes()
    
 #   b = BytesIO()
 #   b.write(bytes([shape[0],shape[1]]))
    


#    data.tofile(f"assets/{name}.b")
    with open(f"assets/{name}.b", "wb") as f:
        f.write(b)
