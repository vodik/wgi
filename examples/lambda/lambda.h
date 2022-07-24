#ifndef __LAMBDA_H__
#define __LAMBDA_H__

#import <stdint.h>

#define _wasm_import_(module, name) __attribute__((import_module(module), import_name(name)))

/* typedef struct event { */
/*     uint32_t id; */
/* } event_t; */

/* void lambda_next(event_t *event) _wasm_import_("lambda0", "lambda_next"); */

uint32_t lambda_event(char *event, uint32_t size) _wasm_import_("lambda0", "lambda_event");
uint32_t lambda_event_size() _wasm_import_("lambda0", "lambda_event_size");

#endif
