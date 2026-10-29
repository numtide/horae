# The Horae server as an OCI image-layout archive (`oci-archive`), loadable
# with `podman load` or `skopeo copy oci-archive:…`. It lives outside
# `nix/packages/`: a new directory there would enter the package source and
# change the default package's hash.
{ horae
, dockerTools
, skopeo
, runCommand
}:
let
  # dockerTools writes the Docker archive format; skopeo converts it below.
  dockerArchive = dockerTools.buildLayeredImage {
    name = "horae";
    tag = horae.version;
    compressor = "none";

    # fakeNss gives the unprivileged user a passwd entry. No CA bundle: the
    # OIDC and Harvest clients use the webpki roots compiled into the binary.
    contents = [ horae dockerTools.fakeNss ];
    extraCommands = ''
      mkdir -m 1777 tmp
    '';

    config = {
      Entrypoint = [ "${horae}/bin/horae" ];
      Cmd = [ "serve" ];
      # The server finds its bundled WASM, JS and CSS in `public/` relative to
      # the working directory, as the NixOS module's WorkingDirectory does.
      WorkingDir = "${horae}/bin";
      User = "65534:65534";
      Env = [
        # A container is reached through its network namespace, not loopback.
        # With this bind the server refuses to start if DEV_LOGIN=1 is passed.
        "HORAE_HOST=0.0.0.0"
        "HORAE_PORT=3000"
      ];
      ExposedPorts."3000/tcp" = { };
    };
  };
in
runCommand "horae-${horae.version}-oci.tar"
{
  nativeBuildInputs = [ skopeo ];
  passthru = { inherit dockerArchive; };
} ''
  skopeo --insecure-policy --tmpdir "$TMPDIR" copy \
    docker-archive:${dockerArchive} \
    oci-archive:$out:horae:${horae.version}
''
