set pagination off

target extended-remote localhost:3333

load

b main
#b hello::hit
continue

monitor rtt setup 0x1ffe8000 30 "SEGGER RTT"
monitor rtt start
monitor rtt server start 19021 0

shell timeout 5

continue

#shell C:/Users/HatzlM/Documents/projects/defmt/defmt-tcp/target/debug/defmt-tcp.exe > test_logs.txt


quit
