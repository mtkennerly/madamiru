This is the raw help text for the command line interface.

## `--help`
```
Play multiple videos at once

Usage: madamiru.exe [OPTIONS] [SOURCES]... [COMMAND]

Commands:
  complete
          Generate shell completion scripts
  schema
          Display schemas that the application uses
  help
          Print this message or the help of the given subcommand(s)

Arguments:
  [SOURCES]...
          Sources to load. Alternatively supports stdin (one value per line)

Options:
      --config <DIRECTORY>
          Use configuration found in DIRECTORY
      --max <MAX>
          How many items to load at most
  -h, --help
          Print help
  -V, --version
          Print version
```

## `complete --help`
```
Generate shell completion scripts

Usage: madamiru.exe complete <COMMAND>

Commands:
  bash
          Completions for Bash
  fish
          Completions for Fish
  zsh
          Completions for Zsh
  powershell
          Completions for PowerShell
  elvish
          Completions for Elvish
  help
          Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

## `schema --help`
```
Display schemas that the application uses

Usage: madamiru.exe schema [OPTIONS] <COMMAND>

Commands:
  config
          Schema for config.yaml
  help
          Print this message or the help of the given subcommand(s)

Options:
      --format <FORMAT>
          [possible values: json, yaml]
  -h, --help
          Print help
```
