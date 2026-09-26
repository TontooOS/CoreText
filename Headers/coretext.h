/*
 * TontooCoreText - C Header
 * TontooOS Crisp Text Framework
 *
 * This header provides C bindings for the CoreText library.
 */

#ifndef TONTOO_CORETEXT_H
#define TONTOO_CORETEXT_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Library version string (borrowed; do not free).
 */
const char *coretext_version(void);

/**
 * Measure `text` at `size` logical px on a `scale` display.
 *
 * Writes logical width/height to `out_w`/`out_h`.
 *
 * @return 1 on success, 0 on null input.
 */
int coretext_measure(const char *text, float size, float scale, float *out_w, float *out_h);

/**
 * Free a string returned by CoreText.
 */
void coretext_string_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif /* TONTOO_CORETEXT_H */
