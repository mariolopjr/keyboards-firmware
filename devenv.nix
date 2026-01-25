{
  pkgs,
  ...
}:

{
  # https://devenv.sh/packages/
  packages = [
    pkgs.qmk
    # pkgs.gcc-arm-embedded
    # pkgs.avrgcc
    # pkgs.avrdude
    # pkgs.dfu-programmer
    # pkgs.dfu-util
    # pkgs.libusb1
  ];

  # https://devenv.sh/languages/
  languages = {
    c.enable = true;
  };

  # https://devenv.sh/scripts/
  scripts = {
    qc.exec = "qmk compile \"$@\"";
    qf.exec = "qmk flash \"$@\"";
  };

  # https://devenv.sh/basics/
  enterShell = ''
    echo "QMK Firmware Development Environment"
    echo "ARM Toolchain: $(arm-none-eabi-gcc --version | head -n 1)"
    echo "AVR Toolchain: $(avr-gcc --version | head -n 1)"
  '';
}
