#!/bin/bash
cd decoder
./test_build.sh
./flash.sh
cd ..

sleep 5 
python -m ectf25.tv.list /dev/ttyACM0
python -m ectf25.tv.subscribe subscription1.bin /dev/ttyACM0
python -m ectf25.tv.subscribe subscription3.bin /dev/ttyACM0
python -m ectf25.tv.subscribe subscription4.bin /dev/ttyACM0
python -m ectf25.tv.list /dev/ttyACM0