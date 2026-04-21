inputs:
inputs.flake-parts.lib.mkFlake { inherit inputs; } {
  systems = import inputs.systems;
  imports = [ ./nix ];
}
