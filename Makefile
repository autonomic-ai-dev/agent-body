.PHONY: all build test check clean

NAME = agent-body

all: build

build:
	cargo build --release -p $(NAME)

test:
	cargo test --release -p $(NAME)

check:
	cargo check -p $(NAME)

clean:
	cargo clean -p $(NAME)
