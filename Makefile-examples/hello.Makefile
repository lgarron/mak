.PHONY: default
default: world

.PHONY: moon
moon:
	echo "hello moon"

.PHONY: world
world: moon
	echo "hello world"
