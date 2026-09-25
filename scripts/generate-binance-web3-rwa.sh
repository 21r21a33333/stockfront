#!/usr/bin/env sh
set -eu

repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
generator_image='openapitools/openapi-generator-cli@sha256:2ab0a9680222de65dc9d3baf861aa02b99e1b80c211d8221ebf3ae8f8a102524'

mkdir -p "$repository_root/crates/binance-web3-rwa"
cp "$repository_root/openapi/binance-web3-rwa.ignore" \
  "$repository_root/crates/binance-web3-rwa/.openapi-generator-ignore"

docker run --rm \
  --user "$(id -u):$(id -g)" \
  --volume "$repository_root:/workspace" \
  "$generator_image" generate \
  --input-spec /workspace/openapi/binance-web3-rwa.yaml \
  --generator-name rust \
  --config /workspace/openapi/binance-web3-rwa-generator.yaml \
  --output /workspace/crates/binance-web3-rwa \
  --ignore-file-override /workspace/openapi/binance-web3-rwa.ignore \
  --global-property apiTests=false,modelTests=false,apiDocs=false,modelDocs=false \
  --skip-validate-spec

# These generic publishing files are not part of Stockfront's vendored client.
rm -f \
  "$repository_root/crates/binance-web3-rwa/.gitignore" \
  "$repository_root/crates/binance-web3-rwa/.travis.yml" \
  "$repository_root/crates/binance-web3-rwa/README.md" \
  "$repository_root/crates/binance-web3-rwa/git_push.sh"

cargo fmt \
  --manifest-path "$repository_root/crates/binance-web3-rwa/Cargo.toml" \
  --package binance-web3-rwa
