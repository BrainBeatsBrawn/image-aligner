#!/usr/bin/env python3

USAGE = """
 Track data going into the image aligner is expected to be in the form of a csv where each line is a single x,y coordinate in the baseline space's unit coordinate space, from 0 to 1 on each axis.
 However, sometimes track data is not in this format, sometimes it is in pixel coordinates within the offset image.
 This program takes an input file, and the width and height of the image it's recorded on, then outputs the file with the new 0-1 coordinate.
 This can then be piped into a new csv file.
 Note that the input csv file should just consist of two columns, both in pixel coordinates, with comma deliniators.
 Also, it is assumed that the coordinates of the pixels are 0-indexed.

 Example usage:
    $ python track-normaliser [path to csv file in pixel coordinates] [image width] [image height]
"""

import sys

if __name__ == "__main__":
    if len(sys.argv) < 4 or sys.argv[1] == "-h" or sys.argv[1] == "--help":
        print(USAGE)
        exit()

    # Parse the width and height as floats.
    width = float(sys.argv[2])
    height = float(sys.argv[3])

    with open(sys.argv[1], 'r') as f:
        for line in f:
            # Preserve headings
            if line.startswith("#"):
                print(line.strip())
            else:
                # Scale the actual coordinates (which should be the first two)
                data = line.split(",")
                print(f"{(0.5 + float(data[0]))/width},{(0.5 + float(data[1]))/height}")

