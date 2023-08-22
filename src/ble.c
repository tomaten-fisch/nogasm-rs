#include "ble.h"

#include "esp_nimble_hci.h"

// NimBLE Client

#include "esp_event.h"
#include "esp_log.h"
#include "esp_nimble_hci.h"
#include "freertos/FreeRTOS.h"
#include "freertos/event_groups.h"
#include "freertos/task.h"
#include "host/ble_gatt.h"
#include "host/ble_hs.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "nvs_flash.h"
#include "sdkconfig.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"
#include <inttypes.h>
#include <stdio.h>

#include "nogasm.h"

// 5730000100234bd4bbd5a6920e4c5653
#define MY_SERVICE_UUID                                                                            \
    BLE_UUID128_DECLARE(0x53, 0x56, 0x4c, 0x0e, 0x92, 0xa6, 0xd5, 0xbb, 0xd4, 0x4b, 0x23, 0x00,    \
                        0x01, 0x00, 0x30, 0x57)
#define MY_WRITE_CHR_UUID                                                                          \
    BLE_UUID128_DECLARE(0x53, 0x56, 0x4c, 0x0e, 0x92, 0xa6, 0xd5, 0xbb, 0xd4, 0x4b, 0x23, 0x00,    \
                        0x02, 0x00, 0x30, 0x57)

#define MIN(a, b) ((a) < (b) ? (a) : (b))

uint16_t conn_handle = BLE_HS_CONN_HANDLE_NONE;
uint16_t write_handle = 0;
bool writable_chr_discovered = false;
static bool service_discovery_in_progress = false;

static char lovense_name[32] = {0};
static char lovense_cmd[32] = {0};
struct ble_hs_adv_fields fields;

uint8_t ble_addr_type;
void ble_app_scan(void);

int write_to_chr(const void *data, uint16_t data_len) {
    if (!writable_chr_discovered) {
        return -9999;
    }
    return ble_gattc_write_no_rsp_flat(conn_handle, write_handle, data, data_len);
    // return ble_gattc_write_flat(conn_handle, write_handle, data, data_len,
    // NULL, NULL);
}

static uint8_t cur_power = 0;
void lovense_set_power(uint8_t power) {
    if (power > 20) {
        return;
    }
    if (cur_power == power) {
        return;
    }
    cur_power = power;
    int len = snprintf(lovense_cmd, 20, "Vibrate:%u;", power);

    int res = write_to_chr(lovense_cmd, len);
    if (res) {
        ESP_LOGE("BLE", "ERROR: ble_gattc_write_no_rsp_flat: %d\n", res);
    }
}

uint8_t lovense_is_connected() { return (uint8_t)writable_chr_discovered; }

const char *lovense_get_name() {
    if (writable_chr_discovered) {
        return lovense_name;
    }
    return "n/a";
}

// Characteristic discovery callback
static int on_disc_chr(uint16_t conn_handle, const struct ble_gatt_error *error,
                       const struct ble_gatt_chr *chr, void *arg) {
    ESP_LOGD("BLE", "CHR_DIS");
    if (error != NULL && error->status != 0) {
        ESP_LOGW("BLE", "Characteristic discovery error %u", error->status);
        return 0;
    }
    if (chr == NULL) {
        ESP_LOGE("BLE", "NULL characteristic");
        return 0;
    }
    if (ble_uuid_cmp((ble_uuid_t *)&chr->uuid, (ble_uuid_t *)MY_WRITE_CHR_UUID) == 0) {
        write_handle = chr->val_handle;
        ESP_LOGI("BLE", "Connected and discovered characteristic");
        writable_chr_discovered = true;
    }

    return 0;
}

// Service discovery callback
static int on_disc_svc(uint16_t conn_handle, const struct ble_gatt_error *error,
                       const struct ble_gatt_svc *service, void *arg) {
    ESP_LOGI("BLE", "SVC_DIS");
    if (error != NULL && error->status != 0) {
        ESP_LOGW("BLE", "Service discovery error: %u", error->status);
        return 0;
    }
    if (service == NULL) {
        ESP_LOGE("BLE", "NULL service");
        return 0;
    }

    int res = ble_gattc_disc_chrs_by_uuid(conn_handle, service->start_handle, service->end_handle,
                                          MY_WRITE_CHR_UUID, on_disc_chr, NULL);
    if (res) {
        ESP_LOGE("BLE", "Failed to start chr discovery for service (error: %d)", res);
    } else {
        ESP_LOGI("BLE", "Discovered a service!");
        ble_gap_disc_cancel();
    }

    service_discovery_in_progress = false;

    return 0;
}

// BLE connection event handling callback
static int ble_gap_connect_cb(struct ble_gap_event *event, void *arg) {
    switch (event->type) {
    case BLE_GAP_EVENT_CONNECT:
        ESP_LOGI("BLE", "GAP EVENT CONNECTED");
        conn_handle = event->connect.conn_handle;
        if (fields.name_len > 0) {
            ESP_LOGI("BLE", "Connected with %.*s", fields.name_len, fields.name);
        }

        vTaskDelay(pdMS_TO_TICKS(1000));

        if (!service_discovery_in_progress) {
            service_discovery_in_progress = true;
            int res = ble_gattc_disc_svc_by_uuid(event->connect.conn_handle, MY_SERVICE_UUID,
                                                 on_disc_svc, NULL);
            if (res != 0) {
                ESP_LOGE("BLE", "Failed to initiate service discovery (error: %d)", res);
                service_discovery_in_progress = false;
            }
        } else {
            ESP_LOGW("BLE", "Service discovery already in progress, skipping.");
        }
        break;
    case BLE_GAP_EVENT_DISCONNECT:
        ESP_LOGI("GAP", "GAP EVENT DISCONNECTED");
        conn_handle = BLE_HS_CONN_HANDLE_NONE;
        writable_chr_discovered = false;
        service_discovery_in_progress = false;
        ble_app_scan();
        break;
    case BLE_GAP_EVENT_CONN_UPDATE_REQ:
        printf("%u, %u, %u, %u, %u, %u\n", event->conn_update_req.peer_params->itvl_min,
               event->conn_update_req.peer_params->itvl_max,
               event->conn_update_req.peer_params->latency,
               event->conn_update_req.peer_params->supervision_timeout,
               event->conn_update_req.peer_params->min_ce_len,
               event->conn_update_req.peer_params->max_ce_len);
        break;
    default:
        ESP_LOGI("BLE", "unhandled GAP_EVENT %u", event->type);
        break;
    }
    return 0;
}

// BLE event handling
static int ble_gap_event(struct ble_gap_event *event, void *arg) {
    struct ble_gap_conn_params conn_params;
    memset(&conn_params, 0, sizeof(conn_params));

    switch (event->type) {
    case BLE_GAP_EVENT_DISC:
        ESP_LOGI("BLE", "GAP EVENT DISCOVERY");
        ble_hs_adv_parse_fields(&fields, event->disc.data, event->disc.length_data);
        if (fields.name_len > 0) {
            ESP_LOGI("BLE", "Discovered %.*s", fields.name_len, fields.name);
            strlcpy(lovense_name, (char *)fields.name, MIN(fields.name_len, sizeof(lovense_name)));
        }
        // printf("Num Services: 16:%u, 32:%u, 128:%u\n", fields.num_uuids16, fields.num_uuids32,
        //        fields.num_uuids128);
        // printf("UUID16 is complete: %u\n", fields.uuids16_is_complete);
        for (int i = 0; i < fields.num_uuids128; i++) {
            // printf("%04x\n", fields.uuids16[0].value);
            if (ble_uuid_cmp((ble_uuid_t *)&fields.uuids128[i], (ble_uuid_t *)MY_SERVICE_UUID) ==
                0) {
                ESP_LOGI("BLE", "FOUND CORRECT DEVICE!");
                ble_gap_disc_cancel();

                conn_params.scan_itvl = BLE_GAP_SCAN_FAST_INTERVAL_MAX;
                conn_params.scan_window = BLE_GAP_SCAN_FAST_WINDOW;
                conn_params.itvl_min = 6; // BLE_GAP_INITIAL_CONN_ITVL_MIN;
                conn_params.itvl_max = 6; // BLE_GAP_INITIAL_CONN_ITVL_MAX;
                conn_params.latency = BLE_GAP_INITIAL_CONN_LATENCY;
                conn_params.supervision_timeout = 500; // BLE_GAP_INITIAL_SUPERVISION_TIMEOUT;
                conn_params.min_ce_len = BLE_GAP_INITIAL_CONN_MIN_CE_LEN;
                conn_params.max_ce_len = BLE_GAP_INITIAL_CONN_MAX_CE_LEN;

                int res = ble_gap_connect(BLE_OWN_ADDR_PUBLIC, &event->disc.addr, 1000,
                                          &conn_params, ble_gap_connect_cb, NULL);
                vTaskDelay(pdMS_TO_TICKS(400));
                if (res != 0) {
                    ESP_LOGE("BLE", "Failed to initiate connection (error: %d)", res);
                }
            }
        }

        break;

    default:
        ESP_LOGI("GAP_EVENT", "unhandled GAP_EVENT %u", event->type);
        break;
    }
    return 0;
}

void ble_app_scan(void) {
    struct ble_gap_disc_params disc_params;

    ESP_LOGI("BLE", "Start scanning ...");

    disc_params.filter_duplicates = 1;
    disc_params.passive = 0;
    disc_params.itvl = 128;
    disc_params.window = 0;
    disc_params.filter_policy = 0;
    disc_params.limited = 0;

    ble_gap_disc(ble_addr_type, BLE_HS_FOREVER, &disc_params, ble_gap_event, NULL);
}

// The application
void ble_app_on_sync(void) {
    ble_hs_id_infer_auto(0, &ble_addr_type);
    vTaskDelay(pdMS_TO_TICKS(1000));
    ble_app_scan();
}

// The infinite task
void host_task(void *param) { nimble_port_run(); }

void init_ble() {
    esp_nimble_hci_init();
    nimble_port_init();
    ble_svc_gap_device_name_set("BLE-Client");
    ble_hs_cfg.sync_cb = ble_app_on_sync;
    ble_svc_gap_init();
    nimble_port_freertos_init(host_task);
}
