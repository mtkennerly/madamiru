# Logging
Log files are stored in the [application folder](/docs/help/application-folder.md).
The latest log file  is named `madamiru_rCURRENT.log`,
and any other log files will be named with a timestamp (e.g., `madamiru_r2000-01-02_03-04-05.log`).

By default, only warnings and errors are logged,
but you can customize this by setting the `RUST_LOG` environment variable
(e.g., `RUST_LOG=madamiru=debug`).
The most recent 5 log files are kept, rotating on app launch or when a log reaches 10 MiB.

You can also enable logging for GStreamer by setting these environment variables:
`GST_DEBUG=3` and `GST_DEBUG_FILE=gst.log`.
