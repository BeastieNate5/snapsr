{
    pkgs ? import <nixpkgs> {}
}:

pkgs.stdenv.mkDerivation rec {
    pname = "snapsr";
    version = "0.0.1";

    src = pkgs.fetchurl {
        url = "https://github.com/BeastieNate5/snapsr/releases/download/v${version}/snapsr-x86_64";
        sha256 = "sha256:c1c8afb0565a9a8111d0111c61a381122ee79a07cdc3b822591fa913c7282a51";
    };

    dontBuild = true;
    dontUnpack = true;

    nativeBuildInputs = [ pkgs.autoPatchelfHook ];

    buildInputs = [ pkgs.glibc pkgs.libgcc ];

    installPhase = ''
        mkdir -p $out/bin
        cp $src $out/bin/snapsr
        chmod +x $out/bin/snapsr
    '';
}
