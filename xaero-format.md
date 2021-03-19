# known
- zip contains `region.xaero` which is uncompressed binary
- per file: 512x512 blocks or one MCRegion
- any 2d-arrays are indexed `[x][z]`, so serialized as z-then-x, unlike MC's usual x-then-z order
- region contains 8x8 `ChunksChunk`s
- chunk contains 4x4 tiles
- tile contains 16x16 `pixels` (block columns)
- pixel contains overlays, blockState, biome, height, light, heightShade, colorType
- overlay contains opacity, overlayBlockState, customColor

## region:
- prelude: `00ff`
- version: `00000001`
- loop over present chunks:
  - byte chunkCoords (`x << 4 | z`)
  - loop over tiles:
    - absent: int -1
    - present: dump pixels

## pixel:
- int pixelParams
- not grass? => int blockState
- overlays? =>
  - byte numOverlays
  - dump overlays
- colorType == 3? => int customColor
- hasBiome? => byte biome

### pixelParams:
- bit 0: notGrass
- bit 1: hasOverlays
- bit 2-3: colorType
- bit 4-5: heightShade
- bit 6: height in extra byte instead of param (not written, legacy?)
- bit 7: isCaveBlock (always false?)
- bit 8-11: light
- bit 12-19: height
- bit 20: hasBiome

### heightShade:
- val 0: normal
- val 1: brighter
- val 2: darker
- val 3: uninitialized (renders as normal)

### colorType:
TODO biomeStuff[0]

### customColor:
TODO biomeStuff[2]

## overlay:
- int overlayParams
- isWater? => int overlayBlockState
- colorType == 2? => int customColour
- opacity > 1? => int opacity

### overlayParams:
- bit 0: notWater
- bit 4: opacity > 1
- bit 4-7: light
- bit 8-11: colorType
