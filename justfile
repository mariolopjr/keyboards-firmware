boards := "m75h m256wh"
target := "thumbv7em-none-eabihf"

# ST system bootloader, as enumerated in DFU mode
dfu_id := "0483:df11"
dfu_addr := "0x08000000"

# agar uses a different chip and bootloader
agar_target := "thumbv7m-none-eabi"
agar_base := "0x08004000"
agar_family := "0x9d5bcf10"

# list available recipes
default:
  @just --list

# build the m75h
[group('m75h')]
m75h *args: (build "m75h" args)

# wait for the m75h in DFU mode, then flash it
[group('m75h')]
m75h-flash: (flash "m75h")

# build the m256wh
[group('m256wh')]
m256wh *args: (build "m256wh" args)

# wait for the m256wh in DFU mode, then flash it
[group('m256wh')]
m256wh-flash: (flash "m256wh")

# build the agar
[group('agar')]
agar *args:
  cd agar && cargo build --release {{ args }}

# package the agar as agar/agar.uf2
[group('agar')]
agar-uf2: agar
  cd agar && cargo objcopy --release -- -O binary agar.bin
  cargo run --release --quiet --manifest-path tools/bin2uf2/Cargo.toml -- \
    agar/agar.bin agar/agar.uf2 --base {{ agar_base }} --family {{ agar_family }}

# build the agar then copy it onto the bootloader volume once it appears
[group('agar')]
agar-flash: agar-uf2
  #!/usr/bin/env bash
  set -euo pipefail
  volumes() {
      for v in /Volumes/*; do
          [ -f "$v/INFO_UF2.TXT" ] && echo "$v"
      done
      return 0
  }
  if [ -z "$(volumes)" ]; then
      just agar-bootloader
      until [ -n "$(volumes)" ]; do
          sleep 1
      done
  fi
  # the marker file does not name the board, and a second UF2 device
  # mounted at the same time would be ambiguous
  if [ "$(volumes | wc -l)" -ne 1 ]; then
      echo 'more than one UF2 volume mounted, unplug the others:' >&2
      volumes >&2
      exit 1
  fi
  vol="$(volumes)"
  echo "==> $vol"
  if cp agar/agar.uf2 "$vol/"; then
      sync
  elif [ -d "$vol" ]; then
      echo 'copy failed' >&2
      exit 1
  else
      echo 'volume detached, the board reset'
  fi

# print how to reach the agar UF2 bootloader
[group('agar')]
agar-bootloader:
  @echo 'RMK firmware: Fn+B'

# build one board
[group('build')]
build board *args: (_board board)
  cd {{ board }} && cargo build --release {{ args }}

# build every board
[group('build')]
all *args:
  #!/usr/bin/env bash
  set -euo pipefail
  for board in {{ boards }}; do
      echo "==> $board"
      (cd "$board" && cargo build --release {{ args }})
  done
  echo "==> agar"
  (cd agar && cargo build --release {{ args }})

# type check without codegen
[group('build')]
check board *args: (_board board)
  cd {{ board }} && cargo check --release {{ args }}

# lint one board
[group('build')]
clippy board *args: (_board board)
  cd {{ board }} && cargo clippy --release {{ args }}

# convert the ELF to a flashable binary at <board>/<board>.bin, with a DFU suffix appended
[group('build')]
bin board: (build board)
  cd {{ board }} && cargo objcopy --release -- -O binary {{ board }}.bin
  cd {{ board }} && dfu-suffix -a {{ board }}.bin

# report section sizes
[group('build')]
size board: (build board)
  cd {{ board }} && cargo size --release -- -A

# remove build artifacts and firmware binaries
[group('build')]
clean:
  #!/usr/bin/env bash
  set -euo pipefail
  for board in {{ boards }}; do
      (cd "$board" && cargo clean)
      rm -f "$board/$board.bin"
  done
  (cd agar && cargo clean)
  rm -f agar/agar.bin agar/agar.uf2
  cargo clean --manifest-path tools/bin2uf2/Cargo.toml

# build, wait for the board in DFU mode, then flash it
[group('flash')]
flash board: (bin board)
  #!/usr/bin/env bash
  set -euo pipefail
  if ! dfu-util -l -d {{ dfu_id }} 2>/dev/null | grep -q 'Found DFU'; then
      just bootloader {{ board }}
      until dfu-util -l -d {{ dfu_id }} 2>/dev/null | grep -q 'Found DFU'; do
          sleep 1
      done
  fi
  cd {{ board }}
  dfu-util -a 0 -d {{ dfu_id }} -s {{ dfu_addr }}:leave -D {{ board }}.bin

# print how to put a board into DFU mode
[group('flash')]
bootloader board: (_board board)
  #!/usr/bin/env bash
  case {{ board }} in
      m75h)   echo 'hold Fn + press Esc' ;;
      m256wh) echo 'hold Fn + press `' ;;
  esac

# check that the host tools are installed
[group('setup')]
doctor:
  #!/usr/bin/env bash
  status=0
  for tool in cargo rustup dfu-util dfu-suffix flip-link; do
      if command -v "$tool" >/dev/null; then
          echo "ok      $tool"
      else
          echo "missing $tool"
          status=1
      fi
  done
  for t in {{ target }} {{ agar_target }}; do
      if rustup target list --installed | grep -qx "$t"; then
          echo "ok      $t"
      else
          echo "missing $t, run \`rustup target add $t\`"
          status=1
      fi
  done
  if cargo objcopy --version >/dev/null 2>&1; then
      echo "ok      cargo-binutils"
  else
      echo "missing cargo-binutils, run \`mise install\`"
      status=1
  fi
  exit $status

[private]
_board board:
  @echo '{{ boards }}' | grep -qw '{{ board }}' || { echo 'unknown board "{{ board }}", expected one of: {{ boards }}' >&2; exit 1; }
