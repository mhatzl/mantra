set pagination off

# see: https://openocd.sourceforge.io/doc/html/GDB-and-OpenOCD.html#GDB-and-OpenOCD
target extended-remote | openocd -c "gdb_port pipe; log_output openocd.log" -f openocd.cfg

load

b main

continue

monitor rtt setup 0x1ffe8000 30 "SEGGER RTT"
monitor rtt start
monitor rtt server start 19021 0

shell timeout 1

continue

quit
