# keyboards-firmware

RMK firmware for my keyboards

- **m75h:** Mode M75H (Sonnet), 75%, STM32F401RC
- **m256wh:** Mode M256WH (Envoy), 65%, STM32F401RC

Each board is a standalone crate

## Toolchain

- rust (pinned in `rust-toolchain.toml`)
- mise (installs `flip-link` and `cargo-binutils`)
- dfu-util
- just

Run `just doctor` to check the host tools

## Build

```sh
just build m75h # or m256wh
just all        # every board
```

Or manually:

```sh
cd m75h # or m256wh
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

## RGB

The m256wh (Envoy) has 30 WS2812 underglow LEDs on PB15. Not supported yet in RMK
