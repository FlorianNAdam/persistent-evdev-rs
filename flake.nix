{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      flake-utils,
      naersk,
      nixpkgs,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        naersk-lib = pkgs.callPackage naersk { };

        persistent-evdev-rs = naersk-lib.buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            systemd
          ];
          postInstall = ''
            mkdir -p $out/etc/udev/rules.d
            cp udev/60-persistent-input-rs-uinput.rules $out/etc/udev/rules.d
          '';
        };
      in
      {
        nixosModules.persistent-evdev-rs =
          {
            config,
            pkgs,
            lib,
            ...
          }:
          let
            cfg = config.services.persistent-evdev-rs;

            settingsFormat = pkgs.formats.json { };

            configFile = settingsFormat.generate "persistent-evdev-rs-config" {
              cache = "/var/cache/persistent-evdev-rs";
              devices = lib.mapAttrs (virt: phys: "/dev/input/by-id/${phys}") cfg.devices;
            };
          in
          {
            options.services.persistent-evdev-rs = {
              enable = lib.mkEnableOption "virtual input devices that persist even if the backing device is hotplugged";

              devices = lib.mkOption {
                default = { };
                type = with lib.types; attrsOf str;
              };
            };

            config = lib.mkIf cfg.enable {
              systemd.services.persistent-evdev-rs = {
                description = "Persistent evdev proxy";
                wantedBy = [ "multi-user.target" ];

                serviceConfig = {
                  Restart = "on-failure";
                  ExecStart = "${pkgs.persistent-evdev-rs}/bin/persistent-evdev-rs ${configFile}";
                  CacheDirectory = "persistent-evdev-rs";
                };
              };

              services.udev.packages = [ pkgs.persistent-evdev-rs ];
            };
          };

        packages = {
          inherit persistent-evdev-rs;
        };

        defaultPackage = self.packages.${system}.persistent-evdev-rs;

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            systemd
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          packages = with pkgs; [
            rust-analyzer
          ];
        };
      }
    );
}
