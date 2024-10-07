#!/bin/bash

repeat_count=2

for ((i=0; i<repeat_count; i++))
do
    port=$((3030 + $i))
    echo "Running ./scripts/test_http.py $i"
    python3 ./scripts/test_http.py $port &
    echo "-------------------------"
done

echo "Completed $repeat_count runs of test.py"
