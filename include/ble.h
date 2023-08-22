#pragma once

#include <stdint.h>

void init_ble();
int write_to_chr(const void *data, uint16_t data_len);
void lovense_set_power(uint8_t power);
