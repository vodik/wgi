CC = clang
LD = llvm-link
OPT = opt

CFLAGS := -std=c99 -Os -flto \
	-D_GNU_SOURCE \
	$(CFLAGS)

OPTFLAGS := -std-link-opts -Os

LDFLAGS := -flto

WASI_SYSROOT = /usr/share/wasi-sysroot

%.bc: %.c
	$(CC) $(CFLAGS) --target=wasm32-unknown-wasi --sysroot=$(WASI_SYSROOT) -S -emit-llvm $(OUTPUT_OPTION) $<

all: hello_world.wasm

all.bc: hello_world.bc
	$(LD) $^ -o $@

opt.bc: all.bc
	$(OPT) $(OPTFLAGS) $? -o $@

hello_world.wasm: opt.bc
	$(CC) $(LDFLAGS) --target=wasm32-unknown-wasi --sysroot=$(WASI_SYSROOT) $? -o $@

clean:
	$(RM) hello_world.wasm *.bc
