## Development
### Prerequisites
Use the latest version of Rust.

On Linux, you'll need some additional system packages.
Refer to [the installation guide](/docs/help/installation.md) for the list.

You'll also need to install GStreamer (tested with 1.22.12).
You can follow the instructions here:
https://github.com/sdroege/gstreamer-rs#installation

### Commands
* Run program:
  * `cargo run`
* Run tests:
  * `cargo test`
* Activate pre-commit hooks (requires Python) to handle formatting/linting:
  ```
  pip install --user pre-commit
  pre-commit install
  ```

### Environment variables
These are optional:

* `MADAMIRU_VERSION`:
  * If set, shown in the window title instead of the Cargo.toml version.
  * Intended for CI.

### Icon
The master icon is `assets/icon.kra`, which you can edit using
[Krita](https://krita.org/en) and then export into the other formats.

### Release preparation
Commands assume you are using [Git Bash](https://git-scm.com) on Windows.

#### Dependencies (one-time)
```bash
pip install invoke
cargo install cargo-lichking

# Verified with commit ba58a5c44ccb7d2e0ca0238d833d17de17c2b53b:
curl -o /c/opt/flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
pip install aiohttp toml
```

Also install the Crowdin CLI tool manually.

#### Process
* Run `invoke prerelease`
  * If you already updated the translations separately,
    then run `invoke prerelease --no-update-lang`
* Update the translation percentages in `src/lang.rs`
* Update the documentation if necessary for any new features.
  Check for any new content that needs to be uncommented (`<!--`).
* Run `git add` for all relevant changes
* Run `invoke release`
  * This will create a new commit/tag and push them.
  * Manually create a release on GitHub and attach the workflow build artifacts
    (plus `dist/*-legal.zip`).
    For Linux and Mac, extract the `.tar.gz` files from the `.zip` files.
* Run `cargo publish`
* Run `invoke release-flatpak`
  * This will automatically push a branch to https://github.com/flathub/com.mtkennerly.madamiru .
  * Manually open a PR for that branch.
  * After the PR is merged, publish via https://buildbot.flathub.org/#/apps/com.mtkennerly.madamiru .
* Run `invoke release-winget`
  * This will automatically push a branch to a fork of https://github.com/microsoft/winget-pkgs .
  * Manually open a pull request for that branch.
