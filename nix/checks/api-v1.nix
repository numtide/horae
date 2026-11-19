# /api/v1 end to end (spec 022, R-14): a Python client generated from the
# OpenAPI document a seeded Horae serves runs the quickstart scenarios, offline.
{ pkgs, perSystem, ... }:
pkgs.runCommand "horae-api-v1"
{
  nativeBuildInputs = with pkgs; [
    bash
    coreutils
    curl
    openapi-generator-cli
    postgresql
    (python3.withPackages (ps: with ps; [ pydantic python-dateutil typing-extensions urllib3 ]))
  ];
  HORAE_TEST_SERVER = "${perSystem.self.default}/bin/horae";
  meta.platforms = pkgs.lib.platforms.linux;
}
  ''
    bash ${./api-v1/run.sh} ${./api-v1/scenarios.py}
    touch "$out"
  ''
