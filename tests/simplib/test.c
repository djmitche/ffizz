#include <stdint.h>
#include "simplib.h"
#include <assert.h>
#include <stdio.h>

int main() {
    assert(add(2, 2) == 4);
    printf("SUCCESS\n");
    return 0;
}
