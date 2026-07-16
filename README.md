# keyboards-firmware

RMK firmware for my keyboards

- **m75h:** Mode M75H (Sonnet), 75%, STM32F401RC
- **m256wh:** Mode M256WH (Envoy), 65%, STM32F401RC

## Toolchain

- rust
- mise
- dfu-util

## Build

```sh
cd m75h
cargo build --release
```

## Flash (USB DFU)

1. Put the board in DFU mode: press **RCTRL + ESC** (the `Bootloader` key on layer 1)
2. Convert and flash:

```sh
cd m75h
cargo objcopy --release -- -O binary m75h.bin
dfu-util -a 0 -d 0483:df11 -s 0x08000000:leave -D m75h.bin
```

The board enumerates as `0483:df11` (ST system bootloader) while in DFU mode

