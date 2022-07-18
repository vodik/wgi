CC = clang
LD = llvm-link
OPT = opt
XXD = xxd

CFLAGS := -std=c99 -Os -flto \
	-D_GNU_SOURCE \
	-DEMSCRIPTEN \
	-DCONFIG_VERSION=\"$(shell cat quickjs/VERSION)\" \
	-DCONFIG_BIGNUM \
	-I./quickjs \
	$(CFLAGS)

OPTFLAGS := -Os

LDFLAGS := -flto

WASI_SYSROOT = /usr/share/wasi-sysroot

%.bc: %.c
	$(CC) $(CFLAGS) --target=wasm32-unknown-wasi --sysroot=$(WASI_SYSROOT) -S -emit-llvm $(OUTPUT_OPTION) $<

.PHONY: all
all: js jsl

bootstrap.h: bootstrap.js
	$(XXD) -i $? $@

quickjs.bc: quickjs/cutils.bc quickjs/libbf.bc quickjs/libregexp.bc quickjs/libunicode.bc quickjs/quickjs.bc
	$(LD) $^ -o $@

js-all.bc: js.bc quickjs.bc quickjs-wasi.bc
	$(LD) $^ -o $@

jsl-all.bc: jsl.bc quickjs.bc quickjs-wasi.bc quickjs-lambda.bc
	$(LD) $^ -o $@

.PHONY: js
js: js.wasm

js-opt.bc: js-all.bc
	$(OPT) $(OPTFLAGS) $? -o $@

js.wasm: js-opt.bc
	$(CC) $(LDFLAGS) --target=wasm32-unknown-wasi --sysroot=$(WASI_SYSROOT) $? -o $@

.PHONY: jsl
jsl: jsl.wasm

jsl-opt.bc: jsl-all.bc
	$(OPT) $(OPTFLAGS) $? -o $@

jsl.wasm: jsl-opt.bc
	$(CC) $(LDFLAGS) --target=wasm32-unknown-wasi --sysroot=$(WASI_SYSROOT) $? -o $@

.PHONY: clean
clean:
	$(RM) js.wasm jsl.wasm *.bc quickjs/*.bc
