{ pkgs, inputs, ... }:
let
  inherit (pkgs) lib stdenv;

  isCross = stdenv.buildPlatform != stdenv.hostPlatform;
  # Use the build machine's packages if cross compiling.
  buildPkgs = if isCross then pkgs.pkgsBuildBuild else pkgs;
  toolchain = with buildPkgs.fenix; combine ([
    (stable.withComponents [
      "rustc"
      "cargo"
      "clippy"
      "rust-std"
      "rust-analyzer"
      "rust-src"
    ])
    targets.wasm32-unknown-unknown.stable.rust-std
  ] ++ lib.optional isCross
    targets.${stdenv.hostPlatform.rust.rustcTarget}.stable.rust-std);
  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (_: toolchain);
  src = lib.cleanSourceWith {
    src = lib.cleanSource ../.;
    filter = path: _type: !(lib.hasSuffix ".nix" path) && !(lib.hasSuffix ".md" path);
  };
  cargoVendorDir = craneLib.vendorCargoDeps { inherit src; };
  common = {
    pname = "horae";
    version = "0.1.0";
    inherit src cargoVendorDir;
    doCheck = false;
    SQLX_OFFLINE = "true";
  };

  # Path patches are dependencies, not dummy workspace code. Dioxus also
  # needs its configuration to select the same server/web profiles and flags.
  dummySrc = craneLib.mkDummySrc {
    inherit src;
    extraDummyScript = ''
      cp -r ${../vendor/dioxus-fullstack-0.7.9}/. "$out/vendor/dioxus-fullstack-0.7.9/"
      cp ${../crates/horae/Dioxus.toml} "$out/crates/horae/Dioxus.toml"
    '';
  };
  dxInputs = with buildPkgs; [ dioxus-cli wasm-pack binaryen ]
    ++ lib.optionals stdenv.hostPlatform.isDarwin [ buildPkgs.darwin.sigtool ];
  dxBuild = ''
    (cd crates/horae && dx build --release --locked)
  '';
  releaseArtifacts = craneLib.buildDepsOnly ((builtins.removeAttrs common [ "src" ]) // {
    inherit dummySrc;
    nativeBuildInputs = dxInputs;
    buildPhaseCargoCommand = dxBuild;
  });
  checkArtifacts = craneLib.buildDepsOnly ((builtins.removeAttrs common [ "src" ]) // {
    inherit dummySrc;
    pname = "horae-checks";
    buildPhaseCargoCommand = ''
      cargo check --locked --workspace --features server --all-targets
      cargo test --locked --workspace --features server --no-run
    '';
  });
in
craneLib.mkCargoDerivation (common // {
  cargoArtifacts = releaseArtifacts;
  doInstallCargoArtifacts = false;
  passthru = { inherit checkArtifacts; };

  # Compile-time sqlx macros (query!, query_as!, …) resolve from the
  # .sqlx/ cache instead of requiring a live database connection.
  SQLX_OFFLINE = "true";

  nativeBuildInputs = dxInputs ++ [
    craneLib.removeReferencesToRustToolchainHook
    craneLib.removeReferencesToVendoredSourcesHook
  ];

  # Use dx build — the same command as development — to compile both the
  # server binary and the WASM client bundle in one step. Crane restores
  # the dependency artifacts and configures vendored sources for offline builds.
  # dx must run from crates/horae/ where Dioxus.toml lives.
  buildPhaseCargoCommand = dxBuild;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    # dx build puts fullstack output under target/dx/{app}/{profile}/web/:
    #   server  — the server binary
    #   public/ — content-addressed WASM, JS, CSS assets + index.html
    local dxdir=target/dx/horae/release/web
    cp "$dxdir/server" $out/bin/horae
    cp -r "$dxdir/public" $out/bin/public
    runHook postInstall
  '';

  meta = {
    description = "A self-hostable time tracking server";
    mainProgram = "horae";
  };
})
