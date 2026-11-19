#!/bin/zsh
cd "$(dirname "$0")/.."
OUT=$(pwd)/images
mkdir -p $OUT
STYLE="Photorealistic, looks like an ordinary smartphone photo taken by a person on the scene, natural colors, slight phone-camera sharpness, no people, no watermark, no text overlays, no brand logos."
typeset -A P
P[crash]="Portrait 3:4 photo. A minor fender bender at a quiet suburban four-way intersection on an overcast day: a silver sedan stopped with its front bumper against the rear bumper of a dark blue compact hatchback, hazard lights on, wet asphalt, stop sign and trees in the background. $STYLE"
P[bumper]="Portrait 3:4 close-up photo of the dented, scratched rear bumper and cracked tail light of a dark blue compact hatchback, with a plain white license plate reading 8KTR214 clearly visible, overcast daylight, shallow depth of field. $STYLE"
P[plate]="Portrait 3:4 photo of the front of a silver sedan with a scuffed front bumper and a plain white license plate reading 7MNV562 clearly visible, parked on wet asphalt, overcast daylight. $STYLE"
P[wide]="Portrait 3:4 wide photo of the same kind of scene: a dark blue compact hatchback and a silver sedan stopped close together after a minor collision at a suburban intersection on an overcast day, with one adult bystander in a grey jacket standing on the far sidewalk looking toward the cars, face visible but small in the frame. Photorealistic, looks like an ordinary smartphone photo, natural colors, no watermark, no text overlays, no brand logos."
P[water]="Portrait 3:4 photo of a brown water stain and bubbling paint spreading across a white kitchen ceiling corner in an apartment, a small drip mark below, indoor daylight. $STYLE"
P[rental]="Portrait 3:4 photo of an empty, bright apartment living room during a move-out inspection: light hardwood floor with a visible long scratch near a large window, bare white walls, afternoon light. $STYLE"
P[hail]="Portrait 3:4 photo taken from a front porch during a sudden hailstorm: large white hailstones piled on a green lawn and bouncing on a dark parked car, heavy grey sky, rain streaks, the neighbor's house across the street with a clearly visible brass house number 1428 beside its front door. $STYLE"
P[rainbow]="Portrait 3:4 photo of a vivid double rainbow arching over a city skyline after rain, wet street in the foreground, dramatic clouds, golden evening light. $STYLE"
P[moose]="Portrait 3:4 photo of a large bull moose standing in the middle of a quiet two-lane forest road at dusk, taken through a car windshield from a safe distance, pine trees on both sides. $STYLE"
names=("$@"); [[ ${#names} -eq 0 ]] && names=(${(k)P})
for n in $names; do
  ( agy -p "Use your image generation tool (Nano Banana) to generate this image and save it as a PNG at $OUT/$n.png: ${P[$n]} Reply with the saved path only." \
      --dangerously-skip-permissions --print-timeout 300s > $OUT/$n.log 2>&1
    echo "$n: $(ls -la $OUT/$n.png 2>/dev/null || echo FAILED)" ) &
done
wait
