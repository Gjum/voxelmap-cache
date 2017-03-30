one .zip per region with a file named `data`

17 byte per column:
1. (4 byte) layer 1: highest partially light blocking block including lava
2. (4 byte) layer 2: seafloor
3. (4 byte) layer 3: highest rain blocking block
4. (4 byte) layer 4: one block above layer 1 ("vegetation")
5. (1 byte) biome ID

4 byte per layer:
1. (1 byte) height
2. (2 byte) blockstate ID
3. (1 byte) blockLight + skyLight*16

https://mods.curse.com/mc-mods/minecraft/225179-voxelmap?page=5#c36

no that's cool I'll detail it.  Was interested to see if you'd get any of it and you did so my curiosity is satisfied :)

so with zero based counting, the last byte (16) is the biomeID.
The first 16 bytes are in 4 groups of 4, each representing a block from a different "layer" (will explain the layers in a moment)

Within those groups the first is the height, from 0-255.
In Java a byte is -128 to 127 so before using it I bitwise-and it with 255 to get the 0-255 value as an int or short or whatever. Same for the rest of the values

the second and third together are the blockstateID, which minecraft gets with
getIdFromBlock(state.getBlock()) + (state.getBlock().getMetaFromState(state) << 12)
Max value for that is 65536, a Short, or two bytes.
I am storing them big endian because my human brain likes that better and I'm handling it explicitly as such and not relying on the platform to figure out big vs little.

the fourth is the light level of the block (blockLight + skyLight*16)
which can be used as an entry into the EntityRenderer.lightmapTexture.getTextureData() array,
which changes as the sun goes up or down and torches flicker etc.

So the layers.
1. highest partially light blocking block (plus lava. lava blocked light up through 1.5 or 1.6 or something).
2. seafloor (when the first layer is water).
3. highest rain blocking block (catches glass blocks and fences up in the air etc).
4. mostly vegetation: one block above the first layer, stuff like flowers, torches, rails etc that don't block anything, but also allows for stuff like fences (on the ground anyway) to show under glass.

The last three layers might or might not exist at a given coordinate (they'll be zerod out if not, leads to nice zip compression).

If during a conversion there's nothing to put into the last three layers, it won't hurt anything, they just won't show on the map.

So, how does Journeymap store things?  I know MapWriter reads (and stores) anvil .mca files.
VoxelMap now has the capability of outputting image files for offline viewing (see the edited description of the mod).
Converting the stored data to an image involves a lot of Minecraft code but I'm sure an offline converter could be done if you really wanted to.
Journeymap cache file to voxelmap cache file would definitely be pretty nice, for people who have explored a lot in that mod!  vice versa would be nice too if people want to convert in the opposite direction

