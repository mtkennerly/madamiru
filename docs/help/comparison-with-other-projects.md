# Comparison with other projects
* [VLC Media Player](https://www.videolan.org/)
  * VLC is simple to use and supports a vast array of video formats.
    However, it is focused on playing one video at a time, which you select manually.
    Of course, you can open multiple VLC windows with different videos,
    but you have to arrange the windows and select the videos yourself.
* [GridPlayer](https://github.com/vzhd1701/gridplayer)
  * GridPlayer is similar to Madamiru.
    However, GridPlayer is designed for the user to manually select videos and curate playlists,
    whereas Madamiru is focused on shuffle play and dynamic selection.
  * GridPlayer does have some shuffle functionality,
    but it's limited to other videos/audio within the same folder
    and does not seem to work with images.
  * GridPlayer only has a single grid,
    whereas Madamiru supports multiple grids with different media sources.

## Performance
On the author's system (Windows 11, AMD Ryzen 9 5900HS w/ 16 cores @ 3.3 GHz, Nvidia GeForce RTX 3070 Mobile, 16 GB RAM),
Madamiru performs better than VLC (3.0.18) and GridPlayer (0.5.3) with several 1080p videos playing at once:

* RAM usage:
  * Madamiru takes about 100 MB per 1080p video.
  * VLC and GridPlayer take about 200 MB per 1080p video.
* Slowdown with nothing else running on the system:
  * Madamiru can handle 10x 1080p videos without any noticeable frame skipping.
    Frame skipping becomes obvious by 16x 1080p videos, but they do continue playing.
  * GridPlayer sometimes has slowdown and hitching around 6~8x 1080p videos, but not always.
    At 9x 1080p videos, all videos visually freeze, although audio continues.
  * VLC doesn't have any frame skipping or slowdown for the first 11x 1080p videos.
    At 12x 1080p videos, all videos visually freeze, although audio continues.
