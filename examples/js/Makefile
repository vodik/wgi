CC = clang
LD = llvm-link
OPT = opt

CFLAGS := -std=c99 -O3 \
	-D_GNU_SOURCE \
	-DEMSCRIPTEN \
	-DCONFIG_VERSION=\"$(shell cat quickjs/VERSION)\" \
	-DCONFIG_BIGNUM \
	-I./quickjs \
	$(CFLAGS)

WASI_SYSROOT = /usr/share/wasi-sysroot

%.bc: %.c
	$(CC) $(CFLAGS) --target=wasm32-unknown-wasi --sysroot=$(WASI_SYSROOT) -S -emit-llvm $(OUTPUT_OPTION) $<

all: js.wasm

all.bc: main.bc quickjs/cutils.bc quickjs/libbf.bc quickjs/libregexp.bc quickjs/libunicode.bc quickjs/quickjs.bc quickjs-wasi.bc
	$(LD) $^ -o $@

opt.bc: all.bc
	$(OPT) -std-link-opts -O3 $? -o $@

js.wasm: opt.bc
	$(CC) -flto --target=wasm32-unknown-wasi --sysroot=$(WASI_SYSROOT) $? -o $@

clean:
	$(RM) js.wasm js.ll *.bc quickjs/*.bc
