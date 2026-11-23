{ pkgs, ... }:
# Kani, the Rust model checker behind `cargo kani -p horae-core`. nixpkgs does
# not package it. This takes the prebuilt release bundle rather than building
# Kani from source: its compiler links one exact nightly's rustc internals, and
# the bundle already ships that build, the precompiled `kani` library and the
# CBMC and Kissat it pins (nixpkgs' CBMC lags the pin, and Kani warns on a
# mismatch). Only the nightly it names is supplied (from fenix) and the binaries
# relinked to it.
#
# The bundle is laid out as `cargo kani setup` leaves `~/.kani/kani-<version>`:
# the driver finds its compiler and libraries next to itself and cargo in
# `toolchain/bin`, so no setup, rustup or `~/.kani` is involved.
let
  inherit (pkgs) lib stdenv;
  version = "0.68.0";
  target = stdenv.hostPlatform.rust.rustcTarget;

  # From the release's rust-toolchain.toml (`nightly-2026-08-21`). No
  # `rustc-dev`: kani-compiler loads `librustc_driver` from `rustc` itself.
  toolchain = pkgs.fenix.combine (with pkgs.fenix.toolchainOf
    {
      channel = "nightly";
      date = "2026-08-21";
      sha256 = "sha256-PDDMZVp1SdCzABXNAy+Unocj2lrQOfZy0EUgu66k520=";
    }; [ rustc cargo rust-std rust-src llvm-tools ]);

  bundle = pkgs.fetchurl {
    url = "https://github.com/model-checking/kani/releases/download/kani-${version}/kani-${version}-${target}.tar.gz";
    hash = {
      x86_64-unknown-linux-gnu = "sha256-MuK0hNc+3gu/ZKLPCHnEJZQiSX2K7E7mfUSN6a54Q9M=";
      aarch64-unknown-linux-gnu = "sha256-AELSaIvxVQJw3vSnnJaq7qE4YxgYy0q1SAEg8tUalCw=";
    }.${target};
  };
in
stdenv.mkDerivation {
  pname = "kani";
  inherit version;
  src = bundle;

  nativeBuildInputs = [ pkgs.autoPatchelfHook pkgs.makeWrapper ];
  buildInputs = [ toolchain stdenv.cc.cc.lib ];

  installPhase = ''
    runHook preInstall
    kani=$out/lib/kani
    mkdir -p $out/bin $kani
    cp -r bin lib library no_core playback rust-toolchain-version rustc-version $kani/
    ln -s ${toolchain} $kani/toolchain
    # Carries CBMC's BSD-4-Clause acknowledgement.
    install -Dm644 license-notes.txt $out/share/doc/kani/license-notes.txt

    # What the kani-verifier launcher does: put the bundle first on PATH, name
    # the driver's mode through argv[0], and pin the toolchain it was built with.
    # cargo links build scripts with `cc`; a dev shell's own `cc` still wins.
    for mode in kani cargo-kani; do
      makeWrapper $kani/bin/kani-driver $out/bin/$mode \
        --argv0 $mode \
        --prefix PATH : $kani/bin:${toolchain}/bin \
        --suffix PATH : ${stdenv.cc}/bin \
        --set RUSTUP_TOOLCHAIN "$(cat rust-toolchain-version)"
    done
    runHook postInstall
  '';

  doInstallCheck = true;
  installCheckPhase = ''
    $out/bin/cargo-kani --version
    $out/bin/kani --version
  '';

  meta = {
    description = "Bit-precise model checker for Rust";
    homepage = "https://github.com/model-checking/kani";
    license = with lib.licenses; [ asl20 mit bsdOriginal ];
    mainProgram = "cargo-kani";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
