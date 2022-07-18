#ifndef __LAMBDA_H__
#define __LAMBDA_H__

#import <stdint.h>

uint32_t lambda_event(char *buf, uint32_t buf_size)
    __attribute__((import_module("lambda0"), import_name("lambda_event")));
uint32_t lambda_event_size()
    __attribute__((import_module("lambda0"), import_name("lambda_event_size")));

int32_t lambda_send_response(const char *buf, uint32_t buf_size)
    __attribute__((import_module("lambda0"), import_name("lambda_send_response")));

#endif
