
import time
import csv
from datetime import datetime as dt
import subprocess

import board
import adafruit_tsl2591


i2c = board.I2C()
sensor = adafruit_tsl2591.TSL2591(i2c)


DELAY = 5

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

    if i == 0:
        return curve[0][1]

    if i == len(curve)-1 and lux >= curve[-1][0]:
        return curve[-1][1]


    # interpolate between 2 points
    return int( curve[i-1][1] + ( (curve[i][1] - curve[i-1][1]) * (lux - curve[i-1][0]) / (curve[i][0] - curve[i-1][0]) ) )


def set_brightness(pct: int):
    pct = int(pct)
    if pct < 0:
        pct = 0
    if pct > 100:
        pct = 100
    subprocess.run(["ddcutil", "--bus=6", "setvcp", "10", str(pct)], check=True,)

lux = sensor.lux
cur_b = target_brightness(lux)
set_brightness(cur_b)


while True:
    time.sleep(DELAY)
    lux = sensor.lux
    target = target_brightness(lux)

    raw = sensor.raw_luminosity

    print(f"{cur_b=}, {target=}, lux={int(lux)}, raw_full={raw[0]}, raw_ir={raw[1]}")
    if cur_b == target:
        continue
    if target > cur_b:
        cur_b += 1
    else:
        cur_b -= 1
    set_brightness(cur_b)
