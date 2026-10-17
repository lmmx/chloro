# Update the blazon-generated badges in README.md using textum, and copy README.md to
# chloro/README.md, which crates.io shows and which must stay identical (`just docs-match`)
emblazon:
    just reinstall
    blazon -b chloro -c chloro
    cp README.md chloro/README.md
