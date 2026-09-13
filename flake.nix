{
  description = "residual — NKP Residuality architecture CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        lib = pkgs.lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Full tree: include_str! embeds skill markdown under src/skills/definitions/.
        src = lib.cleanSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        residual = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "residual";
          version = "0.2.0";
          cargoExtraArgs = "--locked";
          nativeBuildInputs = [ pkgs.git ];

          postInstall = ''
            # fish completions
            mkdir -p $out/share/fish/vendor_completions.d
            $out/bin/residual generate completions \
              > $out/share/fish/vendor_completions.d/residual.fish 2>/dev/null || true

            # man page
            mkdir -p $out/share/man/man1
            $out/bin/residual generate man \
              > $out/share/man/man1/residual.1 2>/dev/null || true
          '';

          meta = with pkgs.lib; {
            description = "NKP Residuality architecture CLI — stressor-driven, attractor-aware, probability-free";
            license = licenses.mit;
            mainProgram = "residual";
          };
        });

        checks = {
          inherit residual;
          residual-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });
          residual-fmt = craneLib.cargoFmt { inherit src; };
          residual-test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
            nativeBuildInputs = [ pkgs.git ];
          });
        };

        apps = {
          default = {
            type = "app";
            program = "${residual}/bin/residual";
          };
          residual = {
            type = "app";
            program = "${residual}/bin/residual";
          };
        };
      in {
        packages.default = residual;
        packages.residual = residual;

        inherit checks apps;

        devShells.default = craneLib.devShell {
          inputsFrom = [ residual ];
          packages = with pkgs; [
            residual
            git
            cargo-watch
            cargo-audit
            cargo-edit
            bun
          ];

          shellHook = ''
            echo "residual dev — \$(command -v residual) on PATH (flake package); use cargo for local rebuilds"
            for tool in agentgrep rtk entire; do
              if ! command -v "$tool" >/dev/null 2>&1; then
                echo "warning: '$tool' not found on PATH — this project's agent conventions expect it (see .claude/CLAUDE.md)" >&2
              fi
            done
          '';
        };

        # Alias so `nix develop .#residual` (as well as the bare `nix develop`)
        # resolves to this project's own pinned build.
        devShells.residual = self.devShells.${system}.default;
      })
    // {
      overlays.default = final: prev: {
        residual = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };

      # Convenience for system/home configs:
      #   inputs.residual.packages.${pkgs.system}.default
      #   inputs.residual.overlays.default
      #   environment.systemPackages = [ inputs.residual.packages.${pkgs.system}.default ];
    };
}
