## Unreleased

* Fixed:
  * Text can now be entered using input method editors.
* Changed:
  * The Mac release is now compiled on Mac OS 14 (ARM) instead of Mac OS 13 (Intel)
    because of [a change by GitHub](https://github.com/actions/runner-images/issues/13046).

## v0.3.0 (2025-08-06)

* Added:
  * There is now a "synchronize" toggle (link icon in the top bar),
    which synchronizes play/pause/seek events for all media of the same category
    (image, audio, video).
  * Each grid now has an icon to view the available media for its configured sources.
  * Several actions now have keyboard shortcuts.
    For more information, [click here](/docs/help/keyboard-controls.md).
  * You can cycle through grids or individual players with the tab key
    and then use keyboard controls on that selection.
* Changed:
  * In the top bar, the playlist actions are now grouped together in a submenu.
* Fixed:
  * Glob sources did not load correctly.
    This was related to v0.2.1's incremental media loading behavior.
  * CLI: When a source was specified, you had to click "add player" for media to start playing.
  * If you tried to add another player when no more media was available,
    the playlist would still be marked as having unsaved changes.
  * If you tried to load an invalid playlist,
    the original playlist would no longer be marked as having unsaved changes.

## v0.2.1 (2025-07-22)

* Fixed:
  * When loading a playlist,
    the application would wait until all sources were scanned before playing any media,
    which could take a while for large collections or slow network folders.
    Now, it will play media as each file is scanned.
* Changed:
  * If the `WGPU_POWER_PREF` environment variable is not set,
    then Madamiru will automatically set it to `high` while running.
    This has fixed application crashes on several users' systems,
    but is ultimately dependent on graphics hardware and drivers.
    If you experience any issues with this, please report it.
  * The standalone Linux release is now compiled on Ubuntu 22.04 instead of Ubuntu 20.04
    because of [a change by GitHub](https://github.com/actions/runner-images/issues/11101).
  * Previously, when loading a playlist,
    if there weren't enough media to fill all of the configured grid slots,
    the application would automatically remove slots that it couldn't fill.
    However, now that slots are filled one-by-one in case the media scan is slow,
    we don't know right away if there's enough valid media to fill all slots.
    Therefore, empty slots will now stay on the grid,
    and you can either remove them manually or reconfigure your sources.
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/madamiru))

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
