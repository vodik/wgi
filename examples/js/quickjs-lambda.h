#ifndef QUICKJS_LAMBDA_H
#define QUICKJS_LAMBDA_H

#include <stdio.h>
#include <stdlib.h>
#import <stdint.h>

#include "quickjs.h"

#ifdef __cplusplus
extern "C" {
#endif

JSModuleDef *js_init_module_lambda(JSContext *ctx, const char *module_name);

#ifdef __cplusplus
} /* extern "C" { */
#endif

#endif /* QUICKJS_LAMBDA_H */
