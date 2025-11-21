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

conf-impact:
    #!/usr/bin/env bash
    BOLD="\033[1m"
    RESET="\033[0m"

    # Header
    echo -e "${BOLD}Score  SizeRank  DiffRank    Impact    Changes      File${RESET}"
    echo "────────────────────────────────────────────────────────────────────────"

    # Collect data with full relative path
    fd -e diff -x bash -c '
        diff_file={}
        base="${diff_file%.diff}"
        chloro="${base}.chloro.rs"

        if [ -f "$chloro" ]; then
            total_lines=$(wc -l < "$chloro")
            diff_lines=$(cat "$diff_file" | rg "^[\+\-]" | grep -vE "^[\+\-][\+\-][\+\-]" | wc -l)

            if [ "$diff_lines" -gt 0 ] && [ "$total_lines" -gt 0 ]; then
                impact=$(awk "BEGIN {printf \"%.1f\", ($diff_lines / $total_lines) * 100}")
                relpath="${diff_file#./chloro-core/tests/conformance/snapshots/}"
                echo "$total_lines $diff_lines $impact ${relpath%.diff}"
            fi
        fi
    ' | {
        data=$(cat)

        # Create size rankings with filename
        size_ranked=$(echo "$data" | sort -k1 -rn | nl -nln | awk '{print $NF, $1, $2, $3, $4}')

        # Create diff rankings with filename
        diff_ranked=$(echo "$data" | sort -k2 -rn | nl -nln | awk '{print $NF, $1, $2, $3, $4}')

        # Join on filename and output
        join -1 1 -2 1 <(echo "$size_ranked" | sort -k1) <(echo "$diff_ranked" | sort -k1) | \
        awk '{
            filename = $1
            size_rank = $2
            total_lines = $3
            diff_lines = $4
            impact = $5
            diff_rank = $6
            score = size_rank * diff_rank
            printf "%5d  %8d  %8d  %7.1f%%  %5d/%-5d  %s\n", score, size_rank, diff_rank, impact, diff_lines, total_lines, filename
        }' | sort -n | head -20
    }
