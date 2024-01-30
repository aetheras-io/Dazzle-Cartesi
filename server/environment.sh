#! /bin/sh

export CC=riscv64-cartesi-linux-gnu-gcc
export CXX=riscv64-cartesi-linux-gnu-g++
export CFLAGS="-march=rv64ima -mabi=lp64"
# export OPENSSL_DIR="/usr/local/ssl"
# export OPENSSL_LIB_DIR="/usr/lib/x86_64-linux-gnu"
# export OPENSSL_INCLUDE_DIR="/usr/include/openssl"
# export PKG_CONFIG_PATH="/usr/lib/x86_64-linux-gnu/pkgconfig/"
