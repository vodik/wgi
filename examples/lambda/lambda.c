#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "jsmn.h"
#include "lambda.h"

int main() {
    /* event_t event; */
    /* lambda_next(&event); */
    /* printf("event: %d!\n", event.id); */

    uint32_t size = lambda_event_size();
    printf("bufsize: %d\n", size);

    char buf[2048];
    uint32_t len = lambda_event(buf, sizeof(buf));
    printf("event: %s!\n", buf);

    jsmn_parser p;
    jsmntok_t t[128];
    jsmn_init(&p);

    int r = jsmn_parse(&p, buf, len, t, sizeof(t) / sizeof(t[0]));
    if (r == JSMN_ERROR_NOMEM) {
        // fprintf(stderr, "No memory\n");
        printf("No memory\n");
    } else if (r < 1 || t[0].type != JSMN_OBJECT) {
        // fprintf(stderr, "Object expected\n");
        printf("Object expected\n");
        return 0;
    }

    for (int i = 1; i < r; i++) {
        if (t[i].type == JSMN_STRING) {
            printf("%.*s ", t[i].end - t[i].start, buf + t[i].start);
        }

        if (t[i].type == JSMN_PRIMITIVE) {
            printf("%.*s ", t[i].end - t[i].start, buf + t[i].start);
        }
    }
}
