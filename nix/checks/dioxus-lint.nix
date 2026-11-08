{ perSystem, ... }:
perSystem.self.default.overrideAttrs (_old: {
  pname = "horae-dioxus-lint";
  # No commit in the version label, so a commit that changes no source keeps this cached.
  HORAE_GIT_REV = "";
  # The crate's few dependencies build faster than the workspace artifacts unpack, and
  # alone they resolve different features, so the shared artifacts would not be reused.
  cargoArtifacts = null;

  # The package source drops Markdown; the self-test checks each rule against the skill.
  DIOXUS_SKILL_DIR = ../../.agents/skills/dioxus-best-practices;

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)
    cargo test --locked -p horae-dioxus-lint
    cargo run --locked -p horae-dioxus-lint
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
