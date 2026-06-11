#include <stdio.h>
#include <assert.h>
#include "flite.h"
cst_voice *register_cmu_us_slt(const char *voxdir);
int main(void) {
    flite_init();
    cst_voice *v = register_cmu_us_slt(NULL);
    if (!v) { printf("voice registration FAILED\n"); return 1; }
    cst_wave *w = flite_text_to_wave("hello world", v);
    if (!w) { printf("synthesis returned NULL\n"); return 1; }
    printf("num_samples=%d sample_rate=%d num_channels=%d\n",
           w->num_samples, w->sample_rate, w->num_channels);
    assert(w->num_samples > 0);
    printf("NATIVE OK\n");
    return 0;
}
