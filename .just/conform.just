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
    just conf-impact

conf-plain:
    just conf | sed 's/\x1b\[[0-9;]*m//g'

conf-impact:
    #!/usr/bin/env bash
    BOLD="\033[1m"
    GREEN="\033[32m"
    RED="\033[31m"
    RESET="\033[0m"
    echo -e "\n${BOLD}Top 20 most impacted (by LOC/diff size):${RESET}"

    # Header
    echo -e "${BOLD}Rank  SizeRank  DiffRank    Impact       +   / -       File${RESET}"
    echo "────────────────────────────────────────────────────────────────────────────"

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
                relpath="${diff_file#./chloro-core/tests/conformance/snapshots/ra/}"
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

            # Format with commas
            cmd_diff = "echo " diff_lines " | numfmt --grouping"
            cmd_total = "echo " total_lines " | numfmt --grouping"
            cmd_diff | getline diff_fmt
            cmd_total | getline total_fmt
            close(cmd_diff)
            close(cmd_total)

            printf "%d|%d|%d|%.1f|%s|%s|%s\n",
                score, size_rank, diff_rank, impact,
                diff_fmt, total_fmt,
                filename
        }' | sort -n | head -20 | nl -nln | awk -F'|' '{
            printf "%4d  %8s  %8s  %7s%%  \033[32m%6s\033[0m  /  \033[31m%-6s\033[0m  %s\n",
                $1, $2, $3, $4, $5, $6, $7
        }'
    }

conf-md:
    #!/usr/bin/env bash
    output=$(just conf-plain)

    # Extract the summary line
    summary=$(echo "$output" | head -1)
    added=$(echo "$summary" | grep -oP '\+\K[0-9,]+')
    removed=$(echo "$summary" | grep -oP '(?<=-)[0-9,]+' | head -1)

    echo "**Summary:** +$added / -$removed"
    echo

    # Build multiline added block with backticks per line
    added_block=$(
        echo "$output" | awk '/^Top 5 added lines:/,/^Top 5 removed lines:/' \
        | grep -E '^[[:space:]]*[0-9]' | head -5 \
        | awk '{count=$1; $1=""; line=$0; sub(/^[ \t]+/, "", line); printf "`%s` × %s<br>", line, count}'
    )

    # Build multiline removed block with backticks per line
    removed_block=$(
        echo "$output" | awk '/^Top 5 removed lines:/,/^Top 20 most impacted/' \
        | grep -E '^[[:space:]]*[0-9]' | head -5 \
        | awk '{count=$1; $1=""; line=$0; sub(/^[ \t]+/, "", line); printf "`%s` × %s<br>", line, count}'
    )

    echo "| **Top 5 <del>Removed</del> Lines** | **Top 5 <ins>Added</ins> Lines** |"
    echo "|---|---|"
    echo "| $removed_block | $added_block |"
    echo

    echo "### Top 20 Most Impacted Files"
    echo
    echo "| Rank | Size Rank | Diff Rank | Impact | Added | Removed | File |"
    echo "|------|-----------|-----------|--------|-------|---------|------|"
    echo "$output" | awk '/^────────────────────────────────────────────────────────────────────────────$/,0' \
        | tail -n +2 | grep -E '^[[:space:]]*[0-9]' | head -20 \
        | awk '{printf "| %s | %s | %s | %s | %s | %s | `%s` |\n", $1, $2, $3, $4, $5, $7, $8}'
