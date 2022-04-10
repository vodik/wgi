#include <stdlib.h>
#include <stdio.h>
#include <string.h>

int main() {
  const char *method = getenv("REQUEST_METHOD");
  const char *path = getenv("PATH_INFO");
  
  const char *greeting = NULL;

  if (strcmp(method, "GET") != 0) {
    printf("Status: 405\n\nMethod Not Allowed\n");
    return 0;
  } else if (strcmp(path, "/hello") == 0) {
    greeting = "Hello";
  } else if (strcmp(path, "/goodbye") == 0) {
    greeting = "Goodbye";
  } else {
    printf("Status: 404\n\nNot Found\n");
    return 0;
  }

  if (!greeting) {
    return 1;
  }

  const char *user_agent = getenv("HTTP_USER_AGENT");
  if (!user_agent) {
    user_agent = "User";
  }
  printf("%s %s!\n", greeting, user_agent);
}
