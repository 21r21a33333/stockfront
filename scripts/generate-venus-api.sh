#!/usr/bin/env sh
set -eu

repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
generator_image='openapitools/openapi-generator-cli@sha256:2ab0a9680222de65dc9d3baf861aa02b99e1b80c211d8221ebf3ae8f8a102524'

mkdir -p "$repository_root/crates/venus-api"
cp "$repository_root/openapi/venus-api.ignore" \
  "$repository_root/crates/venus-api/.openapi-generator-ignore"

docker run --rm \
  --user "$(id -u):$(id -g)" \
  --volume "$repository_root:/workspace" \
  "$generator_image" generate \
  --input-spec /workspace/openapi/venus-api.yaml \
  --generator-name rust \
  --config /workspace/openapi/venus-api-generator.yaml \
  --output /workspace/crates/venus-api \
  --ignore-file-override /workspace/openapi/venus-api.ignore \
  --global-property apiTests=false,modelTests=false,apiDocs=false,modelDocs=false \
  --skip-validate-spec

rm -f \
  "$repository_root/crates/venus-api/.gitignore" \
  "$repository_root/crates/venus-api/.travis.yml" \
  "$repository_root/crates/venus-api/README.md" \
  "$repository_root/crates/venus-api/git_push.sh"

cargo clippy --fix \
  --manifest-path "$repository_root/Cargo.toml" \
  --package venus-api \
  --allow-dirty \
  --allow-staged \
  -- -D warnings

cargo fmt \
  --manifest-path "$repository_root/crates/venus-api/Cargo.toml" \
  --package venus-api
