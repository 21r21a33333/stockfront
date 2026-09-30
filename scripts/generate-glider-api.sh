#!/usr/bin/env sh
set -eu

repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
generator_image='openapitools/openapi-generator-cli@sha256:2ab0a9680222de65dc9d3baf861aa02b99e1b80c211d8221ebf3ae8f8a102524'
generated_dir=$(mktemp -d "$repository_root/.glider-api-generated.XXXXXX")
generated_name=$(basename "$generated_dir")
trap 'rm -rf "$generated_dir"' EXIT HUP INT TERM

cp "$repository_root/openapi/glider-v2.ignore" \
  "$generated_dir/.openapi-generator-ignore"

docker run --rm \
  --user "$(id -u):$(id -g)" \
  --volume "$repository_root:/workspace" \
  "$generator_image" generate \
  --input-spec /workspace/openapi/glider-v2.json \
  --generator-name rust \
  --config /workspace/openapi/glider-v2-generator.yaml \
  --output "/workspace/$generated_name" \
  --ignore-file-override /workspace/openapi/glider-v2.ignore \
  --global-property apis=Discovery,models,supportingFiles,apiTests=false,modelTests=false,apiDocs=false,modelDocs=false \
  --skip-validate-spec

# Generic publishing files are not part of Stockfront's vendored client.
rm -f \
  "$generated_dir/.gitignore" \
  "$generated_dir/.travis.yml" \
  "$generated_dir/README.md" \
  "$generated_dir/git_push.sh"

mkdir -p "$repository_root/crates/glider-api"
rsync --archive --delete "$generated_dir/" "$repository_root/crates/glider-api/"

cargo clippy --fix \
  --manifest-path "$repository_root/Cargo.toml" \
  --package glider-api \
  --allow-dirty \
  --allow-staged \
  -- -D warnings

cargo fmt \
  --manifest-path "$repository_root/crates/glider-api/Cargo.toml" \
  --package glider-api
