# Media sources
To add media to a group,
you can specify the group's sources
using the gear icon in the group's header controls.

You can configure different kinds of sources:

* A `path` source is the path to a specific file or folder on your computer.
  For folders, the application will look for media directly inside of that folder,
  but not in any of its subfolders.
* A `glob` source lets you specify many files/folders at once using
  [glob patterns](https://en.wikipedia.org/wiki/Glob_(programming)).
  For example, `C:\media\**\*.mp4` would select all MP4 files in any subfolder of `C:\media`.

Tips:

* Relative paths are supported and resolve to the current working directory.
* Sources may begin with a `<playlist>` placeholder,
  which resolves to the location of the active playlist.
  If the playlist is not yet saved, then it resolves to the current working directory.
* Sources may begin with `~`,
  which resolves to your user folder (e.g., `C:\Users\your-name` on Windows).
* For globs, if your file/folder name contains special glob characters,
  you can escape them by wrapping them in brackets.
  For example, to select all MP4 files starting with `[prefix]` (because `[` and `]` are special),
  you can write `[[]prefix[]] *.mp4`.
