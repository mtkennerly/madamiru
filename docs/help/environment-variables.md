# Environment variables
Environment variables can be used to tweak some additional behavior:

* `RUST_LOG`: Configure logging.
  Example: `RUST_LOG=madamiru=debug`
* `MADAMIRU_DEBUG`: If this is set to any value,
  then Madamiru will not detach from the console on Windows in GUI mode.
  It will also print some debug messages in certain cases.
  Example: `MADAMIRU_DEBUG=1`
