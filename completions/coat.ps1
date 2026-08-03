# PowerShell completions for coat.
# Install: coat completions powershell   (writes this next to $PROFILE and
# dot-sources it from there)
# (Scheme/module candidates are filled in dynamically via `coat __complete`.)

Register-ArgumentCompleter -Native -CommandName coat -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $subcommands = [ordered]@{
        'clone'       = 'Clone the schemes repository'
        'update'      = 'Update the schemes repository'
        'list'        = 'List available color schemes'
        'browse'      = 'Scroll through schemes interactively'
        'search'      = 'Search schemes by name or author'
        'set'         = 'Switch to a scheme and apply everywhere'
        'random'      = 'Pick a random scheme, preview, and apply'
        'apply'       = 'Apply the current scheme'
        'docs'        = 'Show setup instructions for an app'
        'completions' = 'Install shell completions'
        'help'        = 'Show help'
    }

    # Everything typed after the command name, minus the partial word that is
    # currently being completed (PowerShell includes it in the AST).
    $words = @($commandAst.CommandElements | Select-Object -Skip 1 | ForEach-Object { $_.ToString() })
    if ($wordToComplete -and $words.Count -gt 0 -and $words[-1] -eq $wordToComplete) {
        if ($words.Count -eq 1) { $words = @() }
        else { $words = @($words[0..($words.Count - 2)]) }
    }
    $sub = @($words | Where-Object { -not $_.StartsWith('-') })[0]

    # Each candidate is a (Text, Description) pair.
    $pairs = @()

    if (-not $sub) {
        $pairs = $subcommands.GetEnumerator() | ForEach-Object {
            [pscustomobject]@{ Text = $_.Key; Desc = $_.Value }
        }
    }
    else {
        switch ($sub) {
            'set' {
                # Silent and side-effect free; prints nothing if the scheme
                # library hasn't been cloned yet.
                $pairs = @(& coat __complete schemes 2>$null | ForEach-Object {
                    [pscustomobject]@{ Text = $_; Desc = 'scheme' }
                })
            }
            { $_ -in 'apply', 'docs' } {
                $pairs = @(& coat __complete modules 2>$null | ForEach-Object {
                    [pscustomobject]@{ Text = $_; Desc = 'module' }
                })
            }
            'completions' {
                $pairs = @(
                    [pscustomobject]@{ Text = 'fish';       Desc = 'Fish shell' }
                    [pscustomobject]@{ Text = 'powershell'; Desc = 'PowerShell' }
                )
            }
        }

        # Flags, offered alongside the positional candidates above.
        if ($sub -in 'list', 'search', 'browse', 'random') {
            $pairs += [pscustomobject]@{ Text = '--dark';  Desc = 'Only dark variant schemes' }
            $pairs += [pscustomobject]@{ Text = '--light'; Desc = 'Only light variant schemes' }
        }
        if ($sub -in 'list', 'search') {
            $pairs += [pscustomobject]@{ Text = '--no-preview'; Desc = 'Skip color swatches' }
        }
        if ($sub -eq 'random') {
            $pairs += [pscustomobject]@{ Text = '--dry'; Desc = 'Preview without applying' }
            $pairs += [pscustomobject]@{ Text = '--yes'; Desc = 'Apply without prompting' }
        }
        if ($sub -in 'set', 'random', 'browse') {
            $pairs += [pscustomobject]@{ Text = '--elevate'; Desc = 'Prompt for admin (logon screen + HKLM keys)' }
        }
        if ($sub -eq 'completions') {
            $pairs += [pscustomobject]@{ Text = '--print'; Desc = 'Print the script instead of installing' }
        }
    }

    # StartsWith rather than -like: scheme names contain characters PowerShell
    # would otherwise treat as wildcards.
    $pairs |
        Where-Object { $_.Text.StartsWith($wordToComplete, [StringComparison]::OrdinalIgnoreCase) } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Text, $_.Text, 'ParameterValue', "$($_.Text) — $($_.Desc)"
            )
        }
}
