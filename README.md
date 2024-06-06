# prlib-dl

prlib-dl is a CLI image downloader for Boris Yeltsin Presidential Library.

## Usage

```sh
prlib-dl url dest_dir [pages]
```

### Examples

```sh
# download a whole document to `gko_1279` directory
prlib-dl https://www.prlib.ru/item/1345359 gko_1279
# download page 1, 4, 5, 6, 7 to `laurentian_codex` directory
prlib-dl https://www.prlib.ru/item/342021 laurentian_codex 1,4-7
```
