boards := "m75h m256wh"
target := "thumbv7em-none-eabihf"

# ST system bootloader, as enumerated in DFU mode
dfu_id := "0483:df11"
dfu_addr := "0x08000000"

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
  if rustup target list --installed | grep -qx '{{ target }}'; then
      echo "ok      {{ target }}"
  else
      echo "missing {{ target }}, run \`rustup target add {{ target }}\`"
      status=1
  fi
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
