set shell := ["bash", "-cu"]

# Crate publish order: deps first, facade last.
crates := "jigs-core jigs-macros jigs-trace jigs-log jigs-map jigs"

# Examples that opt into JIGS_MAP=1 map generation.
examples := "hello http async checkout cf-rag todo-api events"

# List recipes.
default:
    @just --list

# Build the whole workspace.
build:
    cargo build --workspace

# Run all tests.
test:
    cargo test --workspace

# Lint with clippy across all targets.
lint:
    cargo clippy --workspace --all-targets

# Generate map for one example. Usage: just map http
map example:
    JIGS_MAP=1 cargo run -q -p jigs-example-{{example}}

# Generate maps for every example.
map-all:
    for ex in {{examples}}; do echo "--- $ex ---"; JIGS_MAP=1 cargo run -q -p jigs-example-$ex; done

# Run an example, passing extra args. Usage: just run http 127.0.0.1:9000
run example *args="":
    cargo run -p jigs-example-{{example}} -- {{args}}

# Bump workspace + inter-crate dep versions. Usage: just bump patch|minor|major
bump level:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{level}}" in patch|minor|major) ;; *)
        echo "level must be: patch | minor | major"; exit 1 ;;
    esac
    current=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)
    IFS='.' read -r maj min pat <<< "$current"
    case "{{level}}" in
        patch) pat=$((pat + 1)) ;;
        minor) min=$((min + 1)); pat=0 ;;
        major) maj=$((maj + 1)); min=0; pat=0 ;;
    esac
    new="$maj.$min.$pat"
    echo "$current -> $new"
    perl -i -pe "s/^version = \"$current\"/version = \"$new\"/" Cargo.toml
    perl -i -pe "s/version = \"=$current\"/version = \"=$new\"/g" Cargo.toml
    cargo update --workspace >/dev/null
    echo "bumped to $new"

# Publish a single crate. Usage: just release jigs-core
release crate:
    cargo publish -p {{crate}}

# Publish every crate in dependency order, sleeping between publishes so crates.io can index each.
release-all:
    #!/usr/bin/env bash
    set -euo pipefail
    for c in {{crates}}; do
        echo "--- publishing $c ---"
        cargo publish -p $c
        echo "(waiting 20s for crates.io to index $c...)"
        sleep 20
    done
    echo "all published."
