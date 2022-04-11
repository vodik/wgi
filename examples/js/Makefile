CFLAGS = -O2 --target=wasm32-unknown-wasi -std=c99 -I./quickjs -D_GNU_SOURCE -DCONFIG_VERSION=\"$(shell cat quickjs/VERSION)\" -DCONFIG_BIGNUM -DEMSCRIPTEN

js.wasm: main.c quickjs/cutils.c quickjs/libbf.c quickjs/libregexp.c quickjs/libunicode.c quickjs/quickjs.c quickjs-wasi.c
	$(CC) $(CFLAGS) $? -o $@

clean:
	@rm js.wasm
