#include <stdio.h>

int main(void) {
    FILE *sentinel = fopen("native-pass.sentinel", "w");
    if (sentinel == NULL) {
        return 2;
    }
    fputs("native ctest executable ran\n", sentinel);
    fclose(sentinel);
    puts("CTEST_PASS_BODY");
    return 0;
}
