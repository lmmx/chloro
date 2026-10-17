# Rust release workflow using release-plz
ship version="auto":
    #!/usr/bin/env -S echo-comment --shell-flags="-euo pipefail" --color="\\033[38;5;202m"

    # 🔍 Refuse to run if not on master branch or not up to date with origin/master
    branch="$(git rev-parse --abbrev-ref HEAD)"
    if [[ "$branch" != "master" ]]; then
        # ❌ Refusing to run: not on 'master' branch (current: $branch)
        exit 1
    fi
    # 🔍 Fetch master branch
    git fetch origin master
    local_rev="$(git rev-parse HEAD)"
    remote_rev="$(git rev-parse origin/master)"
    # 🔍 Local: $local_rev\n🔍 Remote: $remote_rev
    if [[ "$local_rev" != "$remote_rev" ]]; then
        # ❌ Refusing to run: local master branch is not up to date with origin/master
        # Local HEAD:  $local_rev
        # Origin HEAD: $remote_rev
        # Please pull/rebase to update.
        exit 1
    fi

    if [[ -n "$(git status --porcelain)" ]]; then
        # ❌ Refusing to run: the working tree has changes
        git status --short
        exit 1
    fi

    # 🔍 Dry-run release...
    just publish --dry-run
    # ✅ Dry-run went OK

    # 🔍 Lints, docs and the pre-commit pass on the tree as it is, before any change is made,
    # so that a failure leaves nothing to undo
    just prepush
    just precommit
    if [[ -n "$(git status --porcelain)" ]]; then
        # ❌ The pre-commit pass changed files (the README badges, say): commit them first
        git status --short
        exit 1
    fi

    # From here until the version bump is committed, any failure resets the tree to $local_rev
    committed=false
    restore() {
        if [[ "$committed" == false ]]; then
            git reset --quiet --hard "$local_rev"
        fi
    }
    trap restore EXIT

    if [[ "{{version}}" != "auto" ]]; then
        # 🦀 Explicit version bump with release-plz
        crate_names=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[].name')
        version_strings=""
        for crate in $crate_names; do
          version_strings+="$crate@{{version}} "
        done
        release-plz set-version $version_strings
    else
        # 🦀 Version bump chosen by release-plz from the commits
        release-plz update
    fi
    # The root CHANGELOG.md, written by hand, is the changelog and the source of the release
    # notes; the per-crate changelogs end at 0.7.5, so any release-plz edit to them is undone
    git checkout -- chloro/CHANGELOG.md chloro-core/CHANGELOG.md

    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="chloro") | .version')
    if [[ -z "$(just --justfile "{{ justfile() }}" release-notes "$version")" ]]; then
        # ❌ CHANGELOG.md has no section for $version: move the Unreleased entries under it first
        exit 1
    fi

    cargo check --workspace --all-features
    git add -A
    git commit -m "chore(release): 🦀 Upgrades"
    committed=true

    # 🚀 Push the version bump commit
    git push --no-verify

    # 📦 Publish the crates, and create the tags and GitHub releases
    just publish

    # 📝 Set the notes of the GitHub releases from CHANGELOG.md
    just release-notes-publish "$version"


publish mode="":
    #!/usr/bin/env -S bash -euo pipefail
    git_token=$(gh auth token 2>/dev/null) || git_token=$PUBLISH_GITHUB_TOKEN

    ## 🦀 Let release-plz handle workspace crate tagging
    ## It will create tags like: genson-core-v0.2.1, genson-cli-v0.1.5, etc.
    release-plz release --backend github --git-token $git_token {{mode}}

# Print the section of CHANGELOG.md for VERSION, without its heading, each paragraph and
# list item on one line (GitHub shows a line break in release notes as a break)
release-notes version:
    #!/usr/bin/env -S bash -euo pipefail
    awk -v v="{{ version }}" 'index($0, "## [" v "]") == 1 { p = 1; next } p && /^## \[/ { exit } p' CHANGELOG.md \
        | awk '
            /^$/ { if (buf != "") print buf; buf = ""; print; next }
            /^(#|- |[0-9]+\. )/ { if (buf != "") print buf; buf = $0; next }
            { sub(/^ +/, ""); buf = (buf == "" ? $0 : buf " " $0) }
            END { if (buf != "") print buf }' \
        | sed '/./,$!d'

# Set the notes of the chloro and chloro-core GitHub releases of VERSION from CHANGELOG.md
release-notes-publish version:
    #!/usr/bin/env -S bash -euo pipefail
    notes=$(just --justfile "{{ justfile() }}" release-notes "{{ version }}")
    if [[ -z "${notes}" ]]; then
        echo "CHANGELOG.md has no section for {{ version }}" >&2
        exit 1
    fi
    gh release edit "chloro-v{{ version }}" --notes "${notes}"
    gh release edit "chloro-core-v{{ version }}" --notes "$(printf '%s\n\n%s' \
        'chloro-core is released together with chloro; the notes cover both crates.' "${notes}")"

# For when release-plz isn't working
ship-manual:
    #!/usr/bin/env -S echo-comment --shell-flags="-euo pipefail" --color="\\033[38;5;202m"

    # Bump patch version (with cargo-edit)
    cargo set-version --bump patch

    # Commit and tag
    git add .
    git commit -m "chore(release): 🦀 Upgrades"

    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="chloro") | .version')
    git tag chloro-v${version}

    # Push to remote
    git push
    git push --tags

    # Create GitHub release with changelog
    gh release create chloro-v${version} --title "chloro-v${version}" --notes-file <(sed -n "/## \[${version}\]/,/## \[/p" CHANGELOG.md | sed '1d;$d')

    # Publish
    cargo publish
