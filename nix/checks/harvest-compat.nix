# Harvest API compatibility suite (specs/021-harvest-api-compat, FR-070–FR-072):
# real Harvest API v2 clients, built offline from pinned sources, run against a
# seeded Horae. Clients and their pins are listed in harvest-compat/clients/README.md.
{ pkgs, perSystem, ... }:
let
  go-harvest = pkgs.buildGoModule {
    pname = "harvest-compat-go-harvest";
    version = "0";
    src = ./harvest-compat/clients/go-harvest;
    vendorHash = "sha256-/Fw4GiP8VFa9JXlbTCGN3Ni73y+sADUKhoCXdQVhhgM=";
  };
in
pkgs.runCommand "horae-harvest-compat"
{
  nativeBuildInputs = with pkgs; [ bash coreutils curl gawk postgresql ];
  HORAE_TEST_SERVER = "${perSystem.self.default}/bin/horae";
  meta.platforms = pkgs.lib.platforms.linux;
}
  ''
    bash ${./harvest-compat/run.sh} ${go-harvest}/bin/go-harvest
    touch "$out"
  ''
