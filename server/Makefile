
# Path and Variables
SHELL := $(shell which bash)
ORG := aetheras-io
PROJECT := dinder
REPO := github.com/${ORG}/${PROJECT}
ROOT_DIR := $(CURDIR)
TOOLCHAIN_TAG := 0.14.0
TOOLCHAIN_IMG := cartesi/toolchain:$(TOOLCHAIN_TAG)

###########################################################
### Cartesi
debug:
	DAZZLE_DAPP_CONTRACT="0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C" cargo run --bin cartesi mono

build-in-riscv:
	source ./environment.sh ;\
	cargo build -Z build-std=std,core,alloc,panic_abort,proc_macro --target riscv64g-cartesi-linux-gnu.json --release -p cartesi

build-cartesi:
	@echo "This requires a preset .git-credentials to work.  Intended for CI Usage"
	docker run --rm \
		-e CARGO_NET_GIT_FETCH_WITH_CLI=true \
		-v ~/.git-credentials:/root/.git-credentials \
		-v ~/.cargo/git:/usr/local/cargo/git \
		-v ~/.cargo/registry:/usr/local/cargo/registry \
		-u root \
		-v ${ROOT_DIR}:/app \
		-v ${ROOT_DIR}/target:/app/target \
		-w /app \
		${TOOLCHAIN_IMG} /bin/bash -c "git config --global credential.helper store \
			&& make build-in-riscv"
	cp ./target/riscv64g-cartesi-linux-gnu/release/cartesi ../cartesi-dapp/dinder-cartesi-dapp.bin

build-cartesi-env:
	docker run -it --rm \
		-e CARGO_NET_GIT_FETCH_WITH_CLI=true \
		-v ~/.git-credentials:/root/.git-credentials \
		-v ~/.cargo/git:/usr/local/cargo/git \
		-v ~/.cargo/registry:/usr/local/cargo/registry \
		-u root \
		-v ${ROOT_DIR}:/app \
		-v ${ROOT_DIR}/target:/app/target \
		-w /app \
		${TOOLCHAIN_IMG} /bin/bash
