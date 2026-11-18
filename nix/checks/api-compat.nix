{ pkgs, perSystem, ... }:
# Refuse any change to /api/v1 that breaks its clients (research R-3): the
# generated snapshot against the baseline of the last v1 release.
let
  openapi = ../../crates/api/openapi;
  # oasdiff rates removing an optional response field as info, but a client
  # that reads the field breaks all the same.
  levels = pkgs.writeText "oasdiff-severity-levels" ''
    response-optional-property-removed err
  '';
in
pkgs.runCommand "horae-api-compat" { nativeBuildInputs = [ perSystem.self.oasdiff ]; } ''
  oasdiff breaking ${openapi}/v1.baseline.json ${openapi}/v1.json \
    --fail-on ERR --severity-levels ${levels}
  touch $out
''
