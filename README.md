# keyboards-firmware

RMK firmware for my keyboards

- **m75h:** Mode M75H (Sonnet), 75%, STM32F401RC
- **m256wh:** Mode M256WH (Envoy), 65%, STM32F401RC
- **agar:** YDKB Agar, 65%, STM32F103CB

Each board is a standalone crate. The two Mode boards flash over USB DFU and the agar over UF2

## Toolchain

- rust (pinned in `rust-toolchain.toml`, both targets)
- mise (installs `flip-link` and `cargo-binutils`)
- dfu-util (Mode boards only)
- just

Run `just doctor` to check the host tools

## Build

```sh
just build m75h # or m256wh
just agar
just all        # every board
```

Or manually:

```sh
cd m75h # or m256wh or agar
cargo build --release
```

## Flash (USB DFU)

1. Put the board in DFU mode by holding **Fn** and pressing the bootloader key:
   - **m75h:** Fn + Esc
   - **m256wh:** Fn + `

   Fallback: hold the **BOOT0** button on the underside of the PCB
2. Build, convert, and flash:

```sh
just flash m75h    # or m256wh
```

`just flash` blocks until the board shows up in DFU mode

Manually:

```sh
cd m75h # or m256wh
cargo objcopy --release -- -O binary m75h.bin # or m256wh.bin
dfu-util -a 0 -d 0483:df11 -s 0x08000000:leave -D m75h.bin
```

The board enumerates as `0483:df11` (ST system bootloader) while in DFU mode

## Flash (UF2, agar only)

The agar runs a third-party UF2 bootloader in the memory range `0x08000000..0x08004000`. Firmware is linked at `0x08004000` and packaged by `tools/bin2uf2`, because the bootloader's family id `0x9d5bcf10` is not one `cargo-hex-to-uf2` supports

1. Press the flash button to mount the bootloader volume
2. Build, package, and copy:

```sh
just agar-flash
```

`just agar-flash` blocks until a UF2 volume appears

Manually:

```sh
just agar-uf2   # writes agar/agar.uf2
cp agar/agar.uf2 /Volumes/<volume>/
```

## RGB

The m256wh (Envoy) has 30 WS2812 underglow LEDs on PB15. Not supported yet in RMK
