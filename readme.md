# Image Aligner & Tools
This is the repository for the _Image Aligner_, a software to manually map one image onto another.
It is the basis of the dataset alignment phase used in the paper &lt;TODO, A PAPER NEEDS TO BE LINKED HERE&gt;.

What proceeds is [information speficially about the tool](#Tool-Compilation-and-Basic-Usage) (and it's helper tools), building and running them, followed by instructions on using them to recreate Figure 1 in the previous mentioned paper as a "full-usage example".

## Tool Compilation and Basic Usage
The primary program in here (the "Image Aligner") implements a multi-patch image aligner, used to calculate and store the warpmesh required to transform one image (the **offset**) - and an abitrary CSVs of points in that image's coordinate space - into the coordinate space of another (the **baseline**).
It does this by displaying the **offset** image over the top of the **baseline** at varying selectable degrees of transparency, and allowing the user to move the corners of the **offset** image.
Once the user is happy with the initial alignment of the four corners of the image, it can be _sub-divided_ into multiple sub-images, which each can be manipulated to form the _warpmesh_ that maps the **offset** onto the **baseline**.
To aid the user in doing this points that are known to be the same in both images (**correspondence points**) can be marked on both images, and a visual indicator of the distance between them shown.

Also included are a few [tools](tools) that make processing associated data being fed into or taken out of the program a little easier to manage. They are documented on the readme of the folder they exist within, linked.

### Building
The program is written in Rust, which can be attained through a single-line command using [rustup](https://rust-lang.org/tools/install/).

The program can then be compiled with the command:
```
$ cargo build --release
```

All dependencies will be downloaded and compiled.

### Usage

The program can be run by executing the built executable found at `target/release/image-aligner`. Program help can be accessed by passing the `-h` argument:

```
Usage:
  target/debug/track-aligner [OPTIONS] BASELINE_IMAGE OFFSET_IMAGE

Aligns two images through a multi-patch warp. Press 'h' for hotkeys.

Positional arguments:
  BASELINE_IMAGE        Path to the baseline image (the one that will not be
                        warped)
  OFFSET_IMAGE          Path to the offset image (the one that will be warped)

Optional arguments:
  -h,--help             Show this help message and exit
  -s,--start-points START_POINTS
                        Comma-separated list of four points in the baseline
                        image-space that will be used as a starting point for
                        the warp search. Row-major order. Each point is a
                        comma-separated pair of numbers, e.g.
                        "(0,0),(1,0),(0,1),(1,1)" (including quotation marks)
  -i,--input-file INPUT_FILE
                        Path to a file from a previously saved session (saving
                        is done with the 's' key).
  -t,--input-track-file INPUT_TRACK_FILE
                        Path to a track CSV file to load.
  -p,--process-points   If enabled, the program simply immediately processes
                        the track supplied with -t against the supplied
                        pointset and then closes.
  -c,--input-correspondences-file INPUT_CORRESPONDENCES_FILE
                        Path to a .pts correspondences file to load.
```

When in-program, the controls are as follows:
```
   Hotkeys
-------------------------------------------------------------------
q       - Quit.
tab     - subdivide the current warp mesh.
up/down - increase/decrease opacity of the offset image.
1-9     - set the opacity of the offset image to decimal divisions.
s       - Save the current warp mesh.
t       - save the loaded Track in baseline imagespace.
m       - show/hide the warpMesh and correspondence lines.
h       - display this Help text.
b       - stage a correspondence point in Baseline space.
o       - stage a correspondence point in Offset space.
c       - Commit the staged correspondence points.
p       - save the current Point correspondences.
```

## Full Usage Example
![An excerpt from figure 1 of the paper, showing a warp alignment at its early and late stages](docs/figure%20excerpt.png)
*An excerpt from Figure 1 of the paper, showing the final warp alignment at it's early and late stages.*

In the paper, the image aligner is used to align an **offset** generated mosaic of top-down photographs taken and combined to form an image of the *real-world* landscape that *Cataglyphis Velox* ants used to navigate during a previous experiment to a 3D rendered image of the same environment using 3D lidar scan data (the **baseline**).
The generated mosaic image and related ant tracks were sourced from the [CATER dataset](https://www.science.org/doi/10.1126/sciadv.adg2094), while the 3D environment of Seville, Spain, that the experimental work was derived from the [Habitat3D project](https://insectvision.dlr.de/3d-reconstruction-tools/habitat3d).
However, you can find a repository of the combined data [here](link to ORDA). This contains track data that has been normalised to the baseline image coordinates and is convenient.

Here, we will use three files, which can be found within [ORDA](link to ORDA):
| Filename | Description |
|----------|-------------|
| top-down-render(-14,14)-(14,-14).png | An orthographic rendering of the Seville environment as captured in 3D by the Habitat3D project. This rendering was taken such that the top left and bottom right corners have X/Y coordinates *(-14,14)* and *(14,-14)* within the 3D environment (assuming the Z heading is the vertical axis). |
| combined2_opt_dense_alphad.png | A combined top-down image mosaic of the Seville environment, as numerous ants travelled within it, taken from CATER and adjusted to have a transparent background. |
| Ant03R01.csv | Again, from CATER, the lifetime path of an ant within the Seville environment, normalised such that the coordinates are not pixel-indexed to the image mosaic but are indexed from (0,0) to (1,1) - *eg, they are in the combined image's coordinate space* |

- Outline the usage example: Where the data came from, what it is, etc.
- Where to get the data, the optional low-res data

### Opening a Baseline and Offset Image
Initially opening the program with `top-down-render(-14,14)-(14,-14).png` as the **baseline**, and `combined2_opt_dense_alphad.png` as the **offset**, you are presented with the mosaic image overlaid on top of the baseline and streched to completely fill it's area:
![](docs/)


- Opening the baseline iamge without anything, explaining that the image can be moved around
- Opening with the correspondence points, showing that the distance is displayed, and through devision the distances can be decreased
- Opening with the correspondence points and the final warpset, showing the final image.
- Opening all with the **PREPROCESSED** track CSVs


