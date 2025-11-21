import ".just/cargo.just"
import ".just/commit.just"
import ".just/hooks.just"
import ".just/release.just"

default: pc-fix clippy test

conf:
    #!/usr/bin/env bash
    BOLD="\033[1m"
    GREEN="\033[32m"
    RED="\033[31m"
    RESET="\033[0m"

    added=$(fd -e diff -X cat | rg '^+' | wc -l | numfmt --grouping)
    removed=$(fd -e diff -X cat | rg '^- ' | wc -l | numfmt --grouping)

    echo -e "✨📸 Conformance: ${BOLD}${GREEN}+${added}${RESET}/${BOLD}${RED}-${removed}${RESET}"
