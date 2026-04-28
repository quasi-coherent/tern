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
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustPkgs.toolchain;
      };
    };
}
