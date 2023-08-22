
#include <stdint.h>

#include "ble.h"
#include "driver/gpio.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "nogasm.h"
#include "nvs_flash.h"

#define RS_HEAP_SIZE 1 * 1024

#define GPIO_LED GPIO_NUM_2

void app_main(void) {
    nvs_flash_init();
    esp_log_level_set("*", ESP_LOG_INFO);

    uint8_t *rs_heap = malloc(RS_HEAP_SIZE);
    rs_init_heap(rs_heap, RS_HEAP_SIZE);

    RustState rust_state = rs_init();
    // rs_init();

    gpio_set_direction(GPIO_LED, GPIO_MODE_OUTPUT);

    const esp_timer_create_args_t periodic_timer_args = {.callback = &rs_handle_timer,
                                                         .name = "periodic"};

    esp_timer_handle_t periodic_timer;
    ESP_ERROR_CHECK(esp_timer_create(&periodic_timer_args, &periodic_timer));
    ESP_ERROR_CHECK(esp_timer_start_periodic(periodic_timer, 1100));

    init_ble();

    uint16_t ctr = 0;

    while (1) {
        ctr++;
        // vTaskDelay(250 / portTICK_PERIOD_MS);
        // gpio_set_level(GPIO_LED, 1);

        // vTaskDelay(250 / portTICK_PERIOD_MS);
        // gpio_set_level(GPIO_LED, 0);
        uint8_t next_intensity = loop_once(&rust_state);
        lovense_set_power(next_intensity);
        vTaskDelay(1);
        // if ((ctr & 0x00ff) == 0) {
        //     char buf[128];
        //     int len = snprintf(buf, 127, "ctr: %u", ctr);
        //     printf("write_chr: %d\n", write_to_chr(buf, (uint16_t)len));
        // }
    }
}
