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

    echo -e "✨📸 Conformance: ${BOLD}${GREEN}+$added${RESET}/${BOLD}${RED}-$removed${RESET}"

    # Fast top 10 added lines (ignore empty or trivial lines)
    echo -e "\n${BOLD}Top 5 added lines:${RESET}"
    fd -e diff -X cat | rg '^\+' | grep -vE '^\+$' | sort | uniq -c | sort -nr | head -10 | while read count line; do
        # Keep count plain, color only the + sign
        echo -e "$(echo $count | numfmt --grouping)\t${BOLD}${GREEN}${line}${RESET}"
    done

    # Fast top 10 removed lines
    echo -e "\n${BOLD}Top 5 removed lines:${RESET}"
    fd -e diff -X cat | rg '^- ' | grep -vE '^\-$' | sort | uniq -c | sort -nr | head -10 | while read count line; do
        echo -e "$(echo $count | numfmt --grouping)\t${BOLD}${RED}${line}${RESET}"
    done
