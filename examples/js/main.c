#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

#include "quickjs-wasi.h"
#include "quickjs.h"

static JSValue js_print(JSContext *ctx, JSValueConst this_val, int argc,
                        JSValueConst *argv) {
    int i;
    const char *str;

    for (i = 0; i < argc; i++) {
        if (i != 0)
            putchar(' ');
        str = JS_ToCString(ctx, argv[i]);
        if (!str)
            return JS_EXCEPTION;
        fputs(str, stdout);
        JS_FreeCString(ctx, str);
    }
    putchar('\n');
    return JS_UNDEFINED;
}

static int eval_buf(JSContext *ctx, const void *buf, int buf_len,
                    const char *filename, int eval_flags) {
    JSValue val;
    int ret;

    if ((eval_flags & JS_EVAL_TYPE_MASK) == JS_EVAL_TYPE_MODULE) {
        /* for the modules, we compile then run to be able to set
           import.meta */
        val = JS_Eval(ctx, buf, buf_len, filename,
                      eval_flags | JS_EVAL_FLAG_COMPILE_ONLY);
        if (!JS_IsException(val)) {
            js_module_set_import_meta(ctx, val, true);
            val = JS_EvalFunction(ctx, val);
        }
    } else {
        val = JS_Eval(ctx, buf, buf_len, filename, eval_flags);
    }
    if (JS_IsException(val)) {
        js_std_dump_error(ctx);
        ret = -1;
    } else {
        ret = 0;
    }
    JS_FreeValue(ctx, val);
    return ret;
}

static int eval_file(JSContext *ctx, const char *filename) {
    uint8_t *buf;
    int ret, eval_flags;
    size_t buf_len;

    buf = js_load_file(ctx, &buf_len, filename);
    if (!buf) {
        perror(filename);
        exit(1);
    }

    ret = eval_buf(ctx, buf, buf_len, filename, JS_EVAL_TYPE_MODULE);
    js_free(ctx, buf);
    return ret;
}

int main() {
    JSRuntime *rt;
    JSContext *ctx;

    rt = JS_NewRuntime();
    js_std_init_handlers(rt);
    ctx = JS_NewContextRaw(rt);

    JS_AddIntrinsicBaseObjects(ctx);
    JS_AddIntrinsicEval(ctx);

    js_init_module_std(ctx, "std");
    js_init_module_os(ctx, "os");
    JS_SetModuleLoaderFunc(rt, NULL, js_module_loader, NULL);

    JSValue global_obj, console;

    global_obj = JS_GetGlobalObject(ctx);

    console = JS_NewObject(ctx);
    JS_SetPropertyStr(ctx, console, "log",
                      JS_NewCFunction(ctx, js_print, "log", 1));
    JS_SetPropertyStr(ctx, global_obj, "console", console);

    JS_FreeValue(ctx, global_obj);

    const char *path = getenv("PATH_INFO");
    if (path) {
        eval_file(ctx, path + 1);
    } else {
        return 1;
    }

    /* js_std_free_handlers(rt);
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);*/
}
