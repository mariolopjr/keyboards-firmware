# keyboards
 - Mode Designs Sonnet [75%] (2024)
 - Mode Designs Envoy [65%]

## qmk commands
build clangd database: `qmk generate-compilation-database -kb mode/m75h -km mariolopjr`
build sonnet: `qmk compile`
full build sonnet: `qmk compile -kb mode/m75h -km mariolopjr`
build envoy: `qmk compile -kb mode/m256wh`
flash sonnet: `qmk flash`
flash envoy: `qmk flash -kb mode/m256wh`

## update
Quick instructions in updating from upstream
```bash
git checkout main
git pull --rebase upstream main
git pull --tags
git submodule update --init --recursive
git checkout keebs
git merge main
```
