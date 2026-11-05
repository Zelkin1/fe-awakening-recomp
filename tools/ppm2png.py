"""converts the PPM screen dumps Zakuro's boot harness writes in a folder to PNG."""
import pathlib
import sys

from PIL import Image

folder = pathlib.Path(sys.argv[1])
for ppm in sorted(folder.glob("*.ppm")):
    Image.open(ppm).save(ppm.with_suffix(".png"))
    ppm.unlink()
    print("screen", ppm.with_suffix(".png").name)
