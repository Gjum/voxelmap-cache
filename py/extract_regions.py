"""
Takes a source directory containing voxelmap region caches (<x>,<z>.zip)
and creates hardlinks of them in the target directory,
tagged with the source directory's name.

Args: [-v] <main cache> <new cache> [new cache ...]

Input examples:

contribs/
|- foo_2017-01-01/
|  |- play.devotedmc.com/
|     |- world/
|     |  |- Overworld (dimension 0)/
|     |     |- <x>,<z>.zip           <-- this is what we want!
|     |- Overworld (dimension 0)/
|     |  |- <x>,<z>.zip              <-- looks very similar, but is not what we want!
|     |- nether/
|        |- Overworld (dimension 0)/
|           |- <x>,<z>.zip           <-- also not what we want!
|- bar_2017-01-01/
|  |- world/
|     |- Overworld (dimension 0)/
|        |- <x>,<z>.zip
|- baz_2017-01-01_custom/
|  |- Overworld (dimension 0)/
|     |- <x>,<z>.zip

Command example:

python3 extract_regions.py extracted/ contribs/foo_2017-01-01/ contribs/bar_2017-01-01/ contribs/baz_2017-01-01_custom/

Output format:

extracted/
|- <x>,<z>,foo_2017-01-01.zip
|- <x>,<z>,bar_2017-01-01.zip
|- <x>,<z>,baz_2017-01-01_custom.zip

"""
import os
import re
import sys


region_regexp = re.compile('[-0-9]+,[-0-9]+.zip')


def hardlink_cache(main_cache, contrib, src_path, verbose):
    coords = os.path.basename(src_path)[:-4]
    dest_path = '%s/%s,%s.zip' % (main_cache, coords, contrib)
    try:
        os.link(src_path, dest_path)
        mtime = os.path.getmtime(src_path)
        os.utime(dest, (mtime, mtime))
    except FileExistsError:
        if verbose: print('! Skipping existing', src_path, dest_path)
    except Exception as e:
        print('# Error hardlinking', src_path, 'to', dest_path, e.__class__.__name__, e)


def hardlink_dir(main_cache, contrib, contrib_dir, verbose=False, dry=False):
    regions = [r for r in os.listdir(contrib_dir) if r[-4:] == '.zip']
    print('> Found', len(regions), 'regions in', contrib_dir)
    if dry:
        return
    for region in regions:
        hardlink_cache(main_cache, contrib, contrib_dir + '/' + region, verbose)


def hardlink_contrib(main_cache, contrib, contrib_path, verbose=False, dry=False):
    os.makedirs(main_cache, exist_ok=True)
    if verbose: print('Opening', contrib_path)

    entries = [entry.name for entry in os.scandir(contrib_path)]
    if 'world' in entries:
        if verbose: print('? has world dir, skipping Overworld dir if present')
        hardlink_dir(main_cache, contrib, contrib_path + '/world/Overworld (dimension 0)', verbose, dry)
    elif 'Overworld (dimension 0)' in entries:
        if verbose: print('? has only Overworld dir')
        hardlink_dir(main_cache, contrib, contrib_path + '/Overworld (dimension 0)', verbose, dry)
    elif 'mc.civclassic.com' in entries:
        if verbose: print('? has hostname dir')
        if 'world' in [entry.name for entry in os.scandir(contrib_path + '/mc.civclassic.com')]:
            if verbose: print('? has hostname/world dir')
            hardlink_dir(main_cache, contrib+'_world', contrib_path + '/mc.civclassic.com/world/Overworld (dimension 0)', verbose, dry)
        hardlink_dir(main_cache, contrib, contrib_path + '/mc.civclassic.com/Overworld (dimension 0)', verbose, dry)
    elif 'play.devotedmc.com' in entries:
        if verbose: print('? has hostname dir')
        hardlink_dir(main_cache, contrib, contrib_path + '/play.devotedmc.com/world/Overworld (dimension 0)', verbose, dry)
    elif any(region_regexp.match(entry) for entry in entries):
        if verbose: print('? regions were placed directly in contribution')
        hardlink_dir(main_cache, contrib, contrib_path, verbose, dry)
    else:
        if verbose: print('! Skipping: nothing found in', contrib_path)

    if verbose: print('Closing', contrib_path, '\n')


def hardlink_all_contribs(main_cache, *contribs, verbose=False):
    if verbose: print('Hardlinking', len(contribs), 'contributions')
    for contrib_path in contribs:
        contrib = os.path.basename(contrib_path)
        if not contrib:
            contrib = os.path.basename(contrib_path[:-1])
        try:
            hardlink_contrib(main_cache, contrib, contrib_path, verbose)
        except NotADirectoryError:
            if verbose: print('! Skipping non-directory contrib', contrib_path)
        except Exception as e:
            print('# Error processing', contrib_path, e.__class__.__name__, e)


def main(args):
    verbose = False
    if '-v' in args:
        args.remove('-v')
        verbose = True

    try:
        main_cache, *contribs = args
    except ValueError:
        print('Args: [-v] <main cache> <new cache> [new cache ...]')
        return 1

    hardlink_all_contribs(main_cache, *contribs, verbose=verbose)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))

