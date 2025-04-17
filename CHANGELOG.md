## Unreleased

* Changed:
  * The standalone Linux release is now compiled on Ubuntu 22.04 instead of Ubuntu 20.04
    because of [a change by GitHub](https://github.com/actions/runner-images/issues/11101).

## v0.2.0 (2025-03-26)

* Added:
  * When the app can't detect a file's type,
    it will try checking the system's shared MIME database (if available on Linux/Mac),
    and then further fall back to guessing based on the file extension.
  * Partial translations into Brazilian Portuguese, French, German, and Polish.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/madamiru))
* Changed:
  * The app previously used a known set of supported video formats and ignored other video files.
    However, since the exact set depends on which GStreamer plugins you've installed,
    the app will now simply try loading any video file.
  * Application crash and CLI parse errors are now logged.
* Fixed:
  * The `crop` content fit now works correctly for videos.
    Previously, it acted the same as `stretch`.
  * If you drag-and-dropped multiple files into the window
    while there was more than one grid open,
    only one of the files would be inserted into the grid that you selected.
  * If a video is still being downloaded while you watch it,
    the video duration will update as the download continues.

## v0.1.0 (2024-12-12)

* Initial release.
