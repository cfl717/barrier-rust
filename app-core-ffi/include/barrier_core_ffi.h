/**
 * C ABI for app-core (BarrierCore), consumed by Swift / Objective-C / AppKit UI shells.
 *
 * Build the dynamic library:
 *   cargo build -p barrier_core_ffi --release
 *
 * Typical output: target/release/libbarrier_core_ffi.dylib
 *
 * Error handling: functions that return `char *` use NULL for success; non-NULL is an
 * error message allocated by Rust — free with barrier_core_free_string().
 * JSON helpers return a heap string on success — also free with barrier_core_free_string().
 */

#ifndef BARRIER_CORE_FFI_H
#define BARRIER_CORE_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct CoreHandle CoreHandle;

/** Creates a core instance. Returns NULL on failure. */
CoreHandle *barrier_core_new(void);

void barrier_core_free(CoreHandle *handle);

/**
 * Start server. screen_name may be empty (defaults internally).
 * Returns NULL on success, else error string (must free).
 */
char *barrier_core_start_server(CoreHandle *handle, uint16_t port, const char *screen_name);

/**
 * Start client. Both strings may be empty for defaults.
 * Returns NULL on success, else error string (must free).
 */
char *barrier_core_start_client(CoreHandle *handle, const char *server_addr,
                                const char *client_name);

/** Returns NULL on success, else error string (must free). */
char *barrier_core_stop(CoreHandle *handle);

/** JSON status snapshot. Always returns a non-NULL string (must free). */
char *barrier_core_get_status_json(CoreHandle *handle);

/** JSON settings. Always returns a non-NULL string (must free). */
char *barrier_core_get_settings_json(CoreHandle *handle);

/** Frees strings returned by this API. */
void barrier_core_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif /* BARRIER_CORE_FFI_H */
