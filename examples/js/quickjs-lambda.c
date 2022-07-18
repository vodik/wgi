#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cutils.h"
#include "lambda.h"
#include "quickjs-lambda.h"
#include "quickjs.h"

static char *event_buf = NULL;
static size_t event_buf_len = 0;

static JSValue js_lambda_next_event(JSContext *ctx, JSValueConst this_val,
                                    int argc, JSValueConst *argv) {
    uint32_t len = lambda_event_size() + 1;
    if (!len) {
        return JS_TAG_NULL;
    }

    if (len > event_buf_len) {
        event_buf_len = len;
        event_buf = realloc(event_buf, event_buf_len);
    }

    memset(event_buf, 0, event_buf_len);
    uint32_t nbytes_r = lambda_event(event_buf, event_buf_len);
    return JS_ParseJSON(ctx, event_buf, nbytes_r, "<lambda event>");
}

static JSValue js_lambda_send_response(JSContext *ctx, JSValueConst this_val,
                                       int argc, JSValueConst *argv) {
    JSValue json = JS_JSONStringify(ctx, argv[0], JS_NULL, JS_NULL);

    const char *str = JS_ToCString(ctx, json);
    size_t len = strlen(str);

    int err = lambda_send_response(str, len);

    JS_FreeCString(ctx, str);
    return JS_UNDEFINED;
}

static const JSCFunctionListEntry js_lambda_funcs[] = {
    JS_CFUNC_DEF("nextEvent", 0, js_lambda_next_event),
    JS_CFUNC_DEF("sendResponse", 1, js_lambda_send_response),
};

static int js_lambda_init(JSContext *ctx, JSModuleDef *m) {
    return JS_SetModuleExportList(ctx, m, js_lambda_funcs,
                                  countof(js_lambda_funcs));
}

JSModuleDef *js_init_module_lambda(JSContext *ctx, const char *module_name) {
    JSModuleDef *m;
    m = JS_NewCModule(ctx, module_name, js_lambda_init);
    if (!m)
        return NULL;
    JS_AddModuleExportList(ctx, m, js_lambda_funcs, countof(js_lambda_funcs));
    return m;
}
