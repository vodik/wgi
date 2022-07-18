#include <stdbool.h>

#include "bootstrap.h"
#include "quickjs-lambda.h"
#include "quickjs-wasi.h"
#include "quickjs.h"

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

static int eval_bootstrap(JSContext *ctx) {
    const char *buf = (const char *)bootstrap_js;
    return eval_buf(ctx, buf, bootstrap_js_len, "<bootstrap>",
                    JS_EVAL_TYPE_MODULE);
}

int main(int argc, char *argv[]) {
    JSRuntime *rt = JS_NewRuntime();
    js_std_init_handlers(rt);
    JS_SetModuleLoaderFunc(rt, NULL, js_module_loader, NULL);

    JSContext *ctx = JS_NewContext(rt);
    js_std_add_helpers(ctx, argc, argv);
    js_init_module_std(ctx, "std");
    js_init_module_os(ctx, "os");
    js_init_module_lambda(ctx, "lambda");

    int ret = eval_bootstrap(ctx);

    js_std_free_handlers(rt);
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);
    return ret;
}
