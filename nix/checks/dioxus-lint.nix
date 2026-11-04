{ perSystem, ... }:
perSystem.self.default.overrideAttrs (old: {
  pname = "horae-dioxus-lint";
  cargoArtifacts = old.passthru.checkArtifacts;

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
