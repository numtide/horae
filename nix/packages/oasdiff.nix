{ pkgs, ... }:
# oasdiff compares two OpenAPI documents; the api-compat check runs its
# `breaking` command. nixpkgs does not package it.
pkgs.buildGoModule rec {
  pname = "oasdiff";
  version = "1.39.0";

  src = pkgs.fetchFromGitHub {
    owner = "oasdiff";
    repo = "oasdiff";
    rev = "v${version}";
    hash = "sha256-5VJTtW5Fj+eM728bGiHcnB/KJ47Yo8oCU7LzaiAM+x8=";
  };

  vendorHash = "sha256-m3UkwtNVJNFXqhJ0iT1nqcGAA9vsXSQGqan6+cXCKCY=";

  # The tests read fixtures over the network.
  doCheck = false;
  subPackages = [ "." ];
  ldflags = [
    "-s"
    "-w"
    "-X github.com/oasdiff/oasdiff/build.Version=${version}"
  ];

  meta = {
    description = "Compare OpenAPI documents and report breaking changes";
    homepage = "https://github.com/oasdiff/oasdiff";
    license = pkgs.lib.licenses.asl20;
    mainProgram = "oasdiff";
  };
}
