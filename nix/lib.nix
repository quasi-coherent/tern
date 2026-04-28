{ inputs, ... }:
{
  perSystem =
    { pkgs, inputs', ... }:
    let
      rustPkgs = inputs'.fenix.packages.stable;
    in
    {
      _module.args = {
        inherit rustPkgs;

        crane = (inputs.crane.mkLib pkgs).overrideToolchain rustPkgs.toolchain;

        craneWithToolchain = toolchain: (inputs.crane.mkLib pkgs).overrideToolchain toolchain;
      };
    };
}
