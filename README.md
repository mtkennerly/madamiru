# <img src="assets/icon.svg" alt="Logo" width="64" height="64"> Madamiru
Madamiru is a cross-platform media player written in [Rust](https://www.rust-lang.org)
that can automatically shuffle multiple videos, images, and songs at once in a grid layout.

## Features
* Customizable layout with multiple groups of dynamically selected media
* Video formats: AVI, M4V, MKV, MOV, MP4, WebM
* Image formats: BMP, GIF, ICO, JPEG, PNG, TIFF, SVG, WebP
* Audio formats: FLAC, M4A, MP3, WAV
* Subtitles are supported within MKV (but not as separate files)

If you'd like to help translate Madamiru into other languages,
[check out the Crowdin project](https://crowdin.com/project/madamiru).

## Demo
> ![GUI demo](docs/demo-gui.gif)

## Installation
Download the executable for Windows, Linux, or Mac from the
[releases page](https://github.com/mtkennerly/madamiru/releases).
It's portable, so you can simply download it and put it anywhere on your system.

You'll also need to install [GStreamer](https://gstreamer.freedesktop.org/download),
which is a framework that provides various video codecs.

<!--
If you prefer, Madamiru is also available via
[Winget, Scoop, Flatpak, and Cargo](docs/help/installation.md).
-->

Note:

* Windows users may see a popup that says
  "Windows protected your PC",
  because Windows does not recognize the program's publisher.
  Click "more info" and then "run anyway" to start the program.
* Mac users may see a popup that says
  "Madamiru can't be opened because it is from an unidentified developer".
  To allow Madamiru to run, please refer to [this article](https://support.apple.com/en-us/102445),
  specifically the section on `If you want to open an app [...] from an unidentified developer`.

## Usage
Detailed help documentation is available for several topics.

### General
* [Media sources](/docs/help/media-sources.md)

### Interfaces
* [Application folder](/docs/help/application-folder.md)
* [Command line](/docs/help/command-line.md)
* [Configuration file](/docs/help/configuration-file.md)
* [Environment variables](/docs/help/environment-variables.md)
* [Logging](/docs/help/logging.md)

### Other
* [Comparison with other projects](/docs/help/comparison-with-other-projects.md)
* [Troubleshooting](/docs/help/troubleshooting.md)

## Development
Please refer to [CONTRIBUTING.md](./CONTRIBUTING.md).
