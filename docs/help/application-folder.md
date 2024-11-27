# Application folder
Madamiru stores its configuration/logs/etc in the following locations:

* Windows: `%APPDATA%/com.mtkennerly.madamiru`
* Linux: `$XDG_CONFIG_HOME/com.mtkennerly.madamiru` or `~/.config/com.mtkennerly.madamiru`
  * Flatpak: `~/.var/app/com.mtkennerly.madamiru/config/com.mtkennerly.madamiru`
* Mac: `~/Library/Application Support/com.mtkennerly.madamiru`

Alternatively, if you'd like Madamiru to store its configuration in the same
place as the executable, then simply create a file called `madamiru.portable`
in the directory that contains the executable file. You might want to do that
if you're going to run Madamiru from a flash drive on multiple computers.
