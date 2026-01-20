### Image Aligner
The program in here implements a multi-patch image aligner, used to calculate and store the warpmesh required to transform one image (the **offset**) - and an abitrary CSVs of points in that image's coordinate space - into the coordinate space of another (the **baseline**).

## Building
The program is written in Rust and can be compiled with the command:
```
$ cargo build --release
```

## Usage

The program can be run by executing the built executable found at `target/release/track-aligner`. Program help can be accessed by passing the `-h` argument:

```
Usage:
  target/debug/track-aligner [OPTIONS] BASELINE\_IMAGE BASELINE\_SEG OFFSET\_IMAGE OFFSET\_IMAGE\_SEG

Aligns two images through a multi-patch warp. Press 'h' for hotkeys.

Positional arguments:
  BASELINE\_IMAGE        Path to the baseline image (the one that will not be
                        warped)
  BASELINE\_SEG          Path to the baseline bush segmentation image
  OFFSET\_IMAGE          Path to the offset image (the one that will be warped)
  OFFSET\_IMAGE\_SEG      Path to the offset bush segmentation image

Optional arguments:
  -h,--help             Show this help message and exit
  -s,--start-points START\_POINTS
                        Comma-separated list of four points in the baseline
                        image-space that will be used as a starting point for
                        the warp search. Row-major order. Each point is a
                        comma-separated pair of numbers, e.g.
                        "(0,0),(1,0),(0,1),(1,1)" (including quotation marks)
  -i,--input-file INPUT\_FILE
                        Path to a file from a previously saved session (saving
                        is done with the 's' key).
  -t,--input-track-file INPUT\_TRACK\_FILE
                        Path to a track CSV file to load.
  -p,--process-points   If enabled, the program simply immediately processes
                        the track supplied with -t against the supplied
                        pointset and then closes.
```

When in-program, the controls are as follows:
```
   Hotkeys
-------------------------------------------------------------------
q       - Quit.
tab     - Subdivide the current warp mesh.
up/down - Increase/decrease opacity of the offset image.
1-9     - set the opacity of the offset image to decimal divisions.
s       - Save the current warp mesh.
t       - Save the loaded warpset in baseline imagespace.
m       - Show/hide the warpmesh.
h       - Display this text.
```
