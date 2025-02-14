#!/bin/bash
mkdir -p secrets
rm secrets/secrets.json
rm subscription{1,3,4}.bin
python -m ectf25_design.gen_secrets secrets/secrets.json 1 3 4
python -m ectf25_design.gen_subscription secrets/secrets.json subscription1.bin 0xDEADBEEF 0 1000000000000 1
python -m ectf25_design.gen_subscription secrets/secrets.json subscription3.bin 0xDEADBEEF 0 1000000000000 3
python -m ectf25_design.gen_subscription secrets/secrets.json subscription4.bin 0xDEADBEEF 0 1000000000000 4
