#!/bin/sh

set -e

NEXT_TAG=$1
LAST_TAG=$(git describe --tags --abbrev=0)

publishCrate() {
    DIFF=$(git diff --name-only main..$LAST_TAG $1)
    
    if [[ -n "$DIFF" ]]; then
        echo "$1: Changed"
        sed -i -E "0,/version/ s/version = \"[.0-9]+\"/version = \"${NEXT_TAG#v}\"/" $1/Cargo.toml
        sed -i -E "s/$2 = \"[.0-9]+\"/$2 = \"${NEXT_TAG#v}\"/" */Cargo.toml */*/Cargo.toml
        sed -i -E "s/$2 = \{ version = \"[.0-9]+\"/$2 = { version = \"${NEXT_TAG#v}\"/" */Cargo.toml */*/Cargo.toml
        taplo fmt $1/Cargo.toml
        
        cargo fmt --check
        cargo clippy --all-features
        cargo check --all-features
        git commit -a -m "publish $2 $NEXT_TAG"
        for i in {1..3}; do cargo publish -p $2 && break || sleep 10; done
    else
        echo "$1: Unchanged"
    fi
}

publishCrate "yozuk-sdk" "yozuk-sdk"
publishCrate "helpers/english" "yozuk-helper-english"
publishCrate "helpers/preprocessor" "yozuk-helper-preprocessor"
publishCrate "skillset" "yozuk-core-skillset"
publishCrate "yozuk" "yozuk"
publishCrate "yozuk-bundle" "yozuk-bundle"
publishCrate "zuk" "zuk"

git tag $NEXT_TAG