# Fish completions for coat.
# Install: coat completions fish   (writes this to ~/.config/fish/completions/)
# (Scheme/module candidates are filled in dynamically via `coat __complete`.)

# Only offer subcommands when none has been typed yet.
function __coat_needs_command
    set -l cmd (commandline -opc)
    test (count $cmd) -eq 1
end

# Disable file completion by default; commands opt back in where useful.
complete -c coat -f

# Subcommands
complete -c coat -n __coat_needs_command -a clone       -d 'Clone the schemes repository'
complete -c coat -n __coat_needs_command -a update      -d 'Update the schemes repository'
complete -c coat -n __coat_needs_command -a list        -d 'List available color schemes'
complete -c coat -n __coat_needs_command -a browse      -d 'Scroll through schemes interactively'
complete -c coat -n __coat_needs_command -a search      -d 'Search schemes by name or author'
complete -c coat -n __coat_needs_command -a set         -d 'Switch to a scheme and apply everywhere'
complete -c coat -n __coat_needs_command -a random      -d 'Pick a random scheme, preview, and apply'
complete -c coat -n __coat_needs_command -a match       -d 'Build a scheme from the current wallpaper'
complete -c coat -n __coat_needs_command -a apply       -d 'Apply the current scheme'
complete -c coat -n __coat_needs_command -a remove      -d 'Undo the theming of one app'
complete -c coat -n __coat_needs_command -a docs        -d 'Show setup instructions for an app'
complete -c coat -n __coat_needs_command -a completions -d 'Install shell completions'
complete -c coat -n __coat_needs_command -a help        -d 'Show help'

# Scheme-name argument for `set`
complete -c coat -n '__fish_seen_subcommand_from set' -a '(coat __complete schemes)' -d scheme

# App/module argument for `apply` and `docs`
complete -c coat -n '__fish_seen_subcommand_from apply docs remove' -a '(coat __complete modules)' -d module

# Shell argument for `completions`
complete -c coat -n '__fish_seen_subcommand_from completions' -a fish       -d 'Fish shell'
complete -c coat -n '__fish_seen_subcommand_from completions' -a powershell -d 'PowerShell'
complete -c coat -n '__fish_seen_subcommand_from completions' -l print      -d 'Print instead of installing'

# Flags
complete -c coat -n '__fish_seen_subcommand_from list search browse random' -l dark  -d 'Only dark variant schemes'
complete -c coat -n '__fish_seen_subcommand_from list search browse random' -l light -d 'Only light variant schemes'
complete -c coat -n '__fish_seen_subcommand_from list search' -l no-preview -d 'Skip color swatches'
complete -c coat -n '__fish_seen_subcommand_from random match' -l dry -d 'Preview without applying'
complete -c coat -n '__fish_seen_subcommand_from match' -l auto  -d "Let the image's brightness decide (default: dark)"
complete -c coat -n '__fish_seen_subcommand_from match' -l light -d 'Force a light palette'
complete -c coat -n '__fish_seen_subcommand_from match' -l raw -d 'pywal-style: image hues verbatim'
complete -c coat -n '__fish_seen_subcommand_from match' -F -d 'Image to sample'
complete -c coat -n '__fish_seen_subcommand_from remove' -l dry -d 'Show what would be undone'
complete -c coat -n '__fish_seen_subcommand_from remove' -l keep-enabled -d 'Do not touch coat.yaml'
complete -c coat -n '__fish_seen_subcommand_from random' -l yes -s y    -d 'Apply without prompting'
complete -c coat -n '__fish_seen_subcommand_from set browse random' -l elevate -d 'Windows: prompt for admin (logon screen keys)'
