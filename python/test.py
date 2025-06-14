

# SPDX-FileCopyrightText: 2021 ladyada for Adafruit Industries
# SPDX-License-Identifier: MIT

# Simple demo of the TSL2591 sensor.  Will print the detected light value
# every second.
import time
import csv
from datetime import datetime as dt

import board

import adafruit_tsl2591

# Create sensor object, communicating over the board's default I2C bus
i2c = board.I2C()  # uses board.SCL and board.SDA
# i2c = board.STEMMA_I2C()  # For using the built-in STEMMA QT connector on a microcontroller

# Initialize the sensor.
sensor = adafruit_tsl2591.TSL2591(i2c)

# You can optionally change the gain and integration time:
# sensor.gain = adafruit_tsl2591.GAIN_LOW (1x gain)
# sensor.gain = adafruit_tsl2591.GAIN_MED (25x gain, the default)
# sensor.gain = adafruit_tsl2591.GAIN_HIGH (428x gain)
# sensor.gain = adafruit_tsl2591.GAIN_MAX (9876x gain)
# sensor.integration_time = adafruit_tsl2591.INTEGRATIONTIME_100MS (100ms, default)
# sensor.integration_time = adafruit_tsl2591.INTEGRATIONTIME_200MS (200ms)
# sensor.integration_time = adafruit_tsl2591.INTEGRATIONTIME_300MS (300ms)
# sensor.integration_time = adafruit_tsl2591.INTEGRATIONTIME_400MS (400ms)
# sensor.integration_time = adafruit_tsl2591.INTEGRATIONTIME_500MS (500ms)
# sensor.integration_time = adafruit_tsl2591.INTEGRATIONTIME_600MS (600ms)


def sensor_vals():
    now = dt.now()
    raw = sensor.raw_luminosity
    return {
        'date': f"{now.year}-{now.month}-{now.day}",
        'time': f"{now.hour}:{now.minute:02}",
        'lux': sensor.lux,
        'visible': sensor.visible,
        'full': sensor.full_spectrum,
        'infrared': sensor.infrared,
        'raw_full': raw[0],
        'raw_ir': raw[1],
    }



# piece-wise linear brightness curve of (lux, brightness). Values should be sorted by lux
curve = [
    (0, 10),
    (240, 100),
]

def target_brightness(lux):
    i = 0
    for i in range(len(curve)):
        if lux <= curve[i][0]:
            break

    print(f"{i=}")

    if i == 0:
        return curve[0][1]

    if i == len(curve)-1 and lux >= curve[-1][0]:
        return curve[-1][1]


    # interpolate between 2 points
    return curve[i-1][1] + ( (curve[i][1] - curve[i-1][1]) * (lux - curve[i-1][0]) / (curve[i][0] - curve[i-1][0]) )


print(f"{sensor.gain=}, {sensor.integration_time=}, {sensor.raw_luminosity=}")

# exit()


# sleep to move to nearest 5 minutes (+ 1s)
now = dt.now()
cur_cycle = 60 * (now.minute % 5) + now.second

target = 1 + (300 if cur_cycle > 1 else 0)

time.sleep(target - cur_cycle)


try:
    with open('../data/test_vals.csv', 'a') as f:
        fields = ['time', 'lux', 'visible', 'full', 'infrared', 'raw_full','raw_ir', 'date']
        writer = csv.DictWriter(f, fields)

        while True:
            vals = sensor_vals()
            print(vals, f"target={target_brightness(vals['lux'])}")
            writer.writerow(vals)
            time.sleep(5*60)
except:
    ...
