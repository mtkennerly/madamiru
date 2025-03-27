# Troubleshooting
* The window content is way too big and goes off screen.
  * **Linux:** Try setting the `WINIT_X11_SCALE_FACTOR` environment variable to `1`.
    Flatpak installs will have this set automatically.
* The file/folder picker doesn't work.
  * **Steam Deck:** Use desktop mode instead of game mode.
  * **Flatpak:** The `DISPLAY` environment variable may not be getting passed through to the container.
    This has been observed on GNOME systems.
    Try running `flatpak run --nosocket=fallback-x11 --socket=x11 com.mtkennerly.madamiru`.
* The GUI won't launch.
  * There may be an issue with your graphics drivers/support.
    Try using the software renderer instead by setting the `ICED_BACKEND` environment variable to `tiny-skia`.
  * Try forcing the application to use your dedicated GPU instead of the integrated graphics.
    One way to do this is by setting the `WGPU_POWER_PREF` environment variable to `high`.
    Alternatively, on Windows 11, go to: Settings app -> System -> Display -> Graphics.
  * You can try prioritizing different hardware renderers
    by setting the `WGPU_BACKEND` environment variable to `dx12`, `vulkan`, `metal`, or `gl`.
  * **Flatpak:** You can try forcing X11 instead of Wayland:
    `flatpak run --nosocket=wayland --socket=x11 com.mtkennerly.madamiru`
* On Windows, I can't load really long folder/file paths.
  * The application supports long paths,
    but you also need to enable that feature in Windows itself:
    https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation?tabs=registry#registry-setting-to-enable-long-paths
* When I try to play a video, it says `Element failed to change its state`.
  * This probably means that GStreamer is installed,
    but doesn't have the codec necessary for the video.
    You can confirm this by setting two environment variables when you run the application:
    `GST_DEBUG=3` and `GST_DEBUG_FILE=gst.log`,
    and then checking the `gst.log` file for more information.

    If it is indeed a missing codec,
    then you can try installing GStreamer with additional codecs enabled:
    * Windows: You can do this by enabling more features in the GStreamer installer.
    * Ubuntu: `sudo apt-get install -y gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly`
* When I try to play an audio file, it says `Unable to determine media duration` or `end of stream`.
  * This means that the audio backend was unable to handle the file.
    Please check back over time as support for more files may be added/improved

## Environment variables on Windows
Some of the instructions above mention setting environment variables.
If you're using Windows and not familiar with how to do this,
you can follow these instructions:

* Open the Start Menu,
  search for `edit the system environment variables`,
  and select the matching result.
* In the new window, click the `environment variables...` button.
* In the upper `user variables` section, click the `new...` button,
  then enter the variable name and value.
  If the variable already exists, select it and click `edit...`.
