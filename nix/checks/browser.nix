{ pkgs, perSystem, ... }:
pkgs.runCommand "horae-browser-checks"
{
  nativeBuildInputs = with pkgs; [ bash coreutils curl nodejs postgresql ];
  HORAE_TEST_SERVER = "${perSystem.self.default}/bin/horae";
  PLAYWRIGHT_MODULE = "${pkgs.playwright-test}/lib/node_modules/@playwright/test";
  PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
  FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; };
  meta.platforms = pkgs.lib.platforms.linux;
}
  ''
    bash ${../../crates/horae/tests/browser}/run-design-checks.sh
    touch "$out"
  ''
