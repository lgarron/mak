# `mak`

`make`, but `mak` it shorter.

Run `make` with maximum parallelism and get a live overview of the progress.

## Install

```shell
brew install --HEAD lgarron/lgarron/mak
```

## Examples

```shell
git clone https://github.com/cubing/cubing.js && cd cubing.js
make setup

mak test-all
```
![`mak` in action](readme/screenshot.png)

```shell
git clone https://github.com/cubing/cubing.js && cd cubing.js
make quick-setup

mak test-fast
```

<img width="1267" alg="`mak` in action" src="readme/demo.gif">
