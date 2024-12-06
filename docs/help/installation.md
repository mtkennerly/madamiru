# Installation
## Requirements
* Madamiru is available for Windows, Linux, and Mac.
* For the best performance, your system should support one of DirectX, Vulkan, or Metal.
  For other systems, Madamiru will use a fallback software renderer,
  or you can also activate the software renderer by setting the `ICED_BACKEND` environment variable to `tiny-skia`.
* You'll also need to install [GStreamer](https://gstreamer.freedesktop.org/download),
  which is a framework that provides various video codecs.

## Methods
You can install Madamiru one of these ways:

* Download the executable for your operating system from the
  [releases page](https://github.com/mtkennerly/madamiru/releases).
  It's portable, so you can simply download it and put it anywhere on your system.
  **If you're unsure, choose this option.**

<!--
* On Windows, you can use [Winget](https://github.com/microsoft/winget-cli).

  * To install: `winget install -e --id mtkennerly.madamiru`
  * To update: `winget upgrade -e --id mtkennerly.madamiru`

* On Windows, you can use [Scoop](https://scoop.sh).

  * To install: `scoop bucket add extras && scoop install madamiru`
  * To update: `scoop update && scoop update madamiru`

* For Linux, Madamiru is available on [Flathub](https://flathub.org/apps/details/com.mtkennerly.madamiru).
  Note that it has limited file system access by default (`~` and `/run/media`).
  If you'd like to enable broader access, [see here](https://github.com/flathub/com.mtkennerly.madamiru/blob/master/README.md).
-->

* If you have [Rust](https://www.rust-lang.org), you can use Cargo.

  * To install or update: `cargo install --locked madamiru`

  However, note that some features are not yet available in this version.
  The prebuilt release uses a pre-release version of some dependencies,
  which is not possible when installing via Cargo.

  On Linux, this requires the following system packages, or their equivalents
  for your distribution:

  * Ubuntu: `sudo apt-get install -y gcc cmake libx11-dev libxcb-composite0-dev libfreetype6-dev libexpat1-dev libfontconfig1-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav`

## Notes
If you are on Windows:

* When you first run Madamiru, you may see a popup that says
  "Windows protected your PC",
  because Windows does not recognize the program's publisher.
  Click "more info" and then "run anyway" to start the program.

If you are on Mac:

* When you first run Madamiru, you may see a popup that says
  "Madamiru can't be opened because it is from an unidentified developer".
  To allow Madamiru to run, please refer to [this article](https://support.apple.com/en-us/102445),
  specifically the section on `If you want to open an app [...] from an unidentified developer`.
