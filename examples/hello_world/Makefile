hello_world.wasm: hello_world.c
	$(CC) --target=wasm32-unknown-wasi -O2 $< -o $@
	
clean:
	@rm hello_world.wasm