#ifndef BARRIER_CORE_H
#define BARRIER_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void* BarrierCoreHandle;

BarrierCoreHandle barrier_core_new(void);
void barrier_core_free(BarrierCoreHandle handle);

char* barrier_core_start_server(BarrierCoreHandle handle, uint16_t port, const char* screen_name);
char* barrier_core_start_client(
    BarrierCoreHandle handle,
    const char* server_addr,
    const char* client_name
);
char* barrier_core_stop(BarrierCoreHandle handle);
char* barrier_core_get_status_json(BarrierCoreHandle handle);
char* barrier_core_get_settings_json(BarrierCoreHandle handle);
char* barrier_core_switch_client(BarrierCoreHandle handle, const char* client_name, const char* edge);
char* barrier_core_switch_back(BarrierCoreHandle handle);
char* barrier_core_save_settings_json(BarrierCoreHandle handle, const char* json);

void barrier_core_free_string(char* value);

#ifdef __cplusplus
}
#endif

#endif
