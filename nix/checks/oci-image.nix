{ pkgs, flake, system, ... }:
# Daemonless checks on the image archive: OCI layout and media types, runtime
# config, and the files the server needs. The runtime behaviour is covered by
# the oci-image-runtime check.
let
  image = flake.packages.${system}.oci-image;
  horae = flake.packages.${system}.default;
  arch = pkgs.go.GOARCH;
in
pkgs.runCommand "horae-oci-image-check"
{
  nativeBuildInputs = [ pkgs.jq pkgs.gnutar ];
} ''
  blob() { tar -xOf ${image} "blobs/sha256/''${1#sha256:}"; }

  tar -xOf ${image} oci-layout | jq -e '.imageLayoutVersion == "1.0.0"'
  tar -xOf ${image} index.json > index.json
  jq -e '.mediaType == "application/vnd.oci.image.index.v1+json"
    and (.manifests | length) == 1
    and .manifests[0].mediaType == "application/vnd.oci.image.manifest.v1+json"
    and .manifests[0].annotations."org.opencontainers.image.ref.name" == "horae:${horae.version}"' index.json

  blob "$(jq -r '.manifests[0].digest' index.json)" > manifest.json
  jq -e '.config.mediaType == "application/vnd.oci.image.config.v1+json"
    and ([.layers[].mediaType] | unique) == ["application/vnd.oci.image.layer.v1.tar+gzip"]' manifest.json

  blob "$(jq -r '.config.digest' manifest.json)" > config.json
  jq -e '.os == "linux" and .architecture == "${arch}"' config.json
  jq -e '.config.User == "65534:65534"
    and .config.Entrypoint == ["${horae}/bin/horae"]
    and .config.Cmd == ["serve"]
    and .config.WorkingDir == "${horae}/bin"
    and .config.ExposedPorts == {"3000/tcp": {}}
    and (.config.Env | index("HORAE_HOST=0.0.0.0") != null)
    and (.config.Env | index("HORAE_PORT=3000") != null)
    and (.config.Env | map(select(startswith("SSL_CERT_FILE="))) | length) == 1
    and (.config.Env | map(select(test("DEV_LOGIN|SECRET|DATABASE_URL|OIDC|HARVEST"))) | length) == 0' config.json

  # The root filesystem is the union of every layer.
  mkdir rootfs
  for layer in $(jq -r '.layers[].digest' manifest.json); do
    blob "$layer" | tar -xz -C rootfs
  done
  test -x rootfs${horae}/bin/horae
  test -s rootfs${horae}/bin/public/index.html
  test -n "$(ls rootfs${horae}/bin/public/assets | grep -E '\.wasm$')"
  test -n "$(ls rootfs${horae}/bin/public/assets | grep -E '\.js$')"
  test -n "$(ls rootfs${horae}/bin/public/assets | grep -E '\.css$')"
  test -s "rootfs$(jq -r '.config.Env[] | select(startswith("SSL_CERT_FILE=")) | sub("SSL_CERT_FILE="; "")' config.json)"
  # /etc/passwd links into the store by absolute path.
  grep -q '^nobody:x:65534:65534:' "rootfs$(readlink rootfs/etc/passwd)"
  # Extraction applies the umask, so read the mode from the layer itself.
  for layer in $(jq -r '.layers[].digest' manifest.json); do
    blob "$layer" | tar -tvz
  done | grep -Eq '^drwxrwxrwt .* \./tmp/?$'
  # No shell, and no data or secrets baked in.
  test ! -e rootfs/bin/sh
  test -z "$(ls -A rootfs/var 2>/dev/null | grep -v '^empty$' || true)"

  touch $out
''
