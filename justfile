firmware := justfile_directory() / "qmk_firmware"

# run the qmk CLI through uv so it works with or without the venv activated
qmk := "uv run --quiet qmk"

# list available recipes
default:
  @just --list

# compile every build target in qmk.json
[group('build')]
all *args: _require-firmware
  {{ qmk }} userspace-compile {{ args }}

# compile the keyboard from qmk.ini or an override using `just compile mode/m256wh`
[group('build')]
compile keyboard='' *args: _require-firmware
  {{ qmk }} compile {{ if keyboard =~ '^-' { keyboard } else if keyboard != '' { '-kb ' + keyboard } else { '' } }} {{ args }}

# compile and flash the keyboard from qmk.ini or an override using `just flash mode/m256wh`
[group('build')]
flash keyboard='' *args: _require-firmware
  {{ qmk }} flash {{ if keyboard =~ '^-' { keyboard } else if keyboard != '' { '-kb ' + keyboard } else { '' } }} {{ args }}

# regenerate compile_commands.json across every build target, then rebuild so clangd resolves symbols
[group('lsp')]
compiledb: _require-firmware
  #!/usr/bin/env bash
  set -euo pipefail
  scratch=$(mktemp -d)
  trap 'rm -rf "$scratch"' EXIT

  # qmk writes one database per keyboard/keymap and each run starts with a clean
  # generate them all up front and after rebuild
  while read -r keyboard keymap; do
      {{ qmk }} generate-compilation-database -kb "$keyboard" -km "$keymap"
      cp compile_commands.json "$scratch/${keyboard//\//-}-$keymap.json"
  done < <(python3 -c 'import json; [print(kb, km) for kb, km in json.load(open("qmk.json"))["build_targets"]]')

  python3 - "$scratch" <<'PY'
  import json, re, sys
  from pathlib import Path

  seen, merged = set(), []
  databases = sorted(Path(sys.argv[1]).glob("*.json"))
  for database in databases:
      entries = json.loads(database.read_text())

      # qmk includes keymap.c from a generated unit, so clangd may use the same keymap 
      # for all keyboards. ensure each keymap has the flags of the unit that generated it,
      # which is what ensures QMK_KEYBOARD_H points to the right LAYOUT macros
      keymaps = [
          {**entry, "file": match.group(1)}
          for entry in entries
          if entry["file"].endswith("default_keyboard.c")
          if (match := re.search(r'-DKEYMAP_C="([^"]+)"', entry["command"]))
      ]

      # only compile shared files like userspace.c once
      for entry in entries + keymaps:
          if entry["file"] not in seen:
              seen.add(entry["file"])
              merged.append(entry)

  Path("compile_commands.json").write_text(json.dumps(merged, indent=4))
  print(f"merged {len(merged)} compile commands from {len(databases)} build targets")
  PY

  # QMK_KEYBOARD_H points into .build which clean deletes, so re-build it
  {{ qmk }} userspace-compile

# remove build artifacts
[group('build')]
clean: _require-firmware
  {{ qmk }} clean --all

# clone the qmk_firmware submodule
[group('setup')]
setup:
  git submodule update --init --recursive

# check the local toolchain
[group('setup')]
doctor:
  {{ qmk }} doctor

# update qmk_firmware and its submodules to the latest commit
[group('setup')]
update: _require-firmware
  git -C "{{ firmware }}" fetch origin master
  git -C "{{ firmware }}" checkout master
  git -C "{{ firmware }}" merge --ff-only origin/master
  git -C "{{ firmware }}" submodule update --init --recursive
  @echo 'qmk_firmware updated'

# create a new keymap using `just new-keymap mode/m256wh -km experiment`
[group('keymap')]
new-keymap keyboard='' *args:
  {{ qmk }} new-keymap {{ if keyboard =~ '^-' { keyboard } else if keyboard != '' { '-kb ' + keyboard } else { '' } }} {{ args }}

# add a keymap to the build targets in qmk.json
[group('keymap')]
add keyboard='' *args:
  {{ qmk }} userspace-add {{ if keyboard =~ '^-' { keyboard } else if keyboard != '' { '-kb ' + keyboard } else { '' } }} {{ args }}

# remove a keymap from the build targets in qmk.json
[group('keymap')]
remove keyboard='' *args:
  {{ qmk }} userspace-remove {{ if keyboard =~ '^-' { keyboard } else if keyboard != '' { '-kb ' + keyboard } else { '' } }} {{ args }}

# list the build targets in qmk.json
[group('keymap')]
list:
  {{ qmk }} userspace-list

[private]
_require-firmware:
  @test -d "{{ firmware }}/quantum" || { echo 'qmk_firmware is missing, run `just setup`' >&2; exit 1; }
