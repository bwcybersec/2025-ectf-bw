#!/bin/bash
mkdir -p secrets
rm secrets/secrets.json
rm secrets/global.secrets
rm subscription{1,2,3,4}.bin
python -m ectf25_design.gen_secrets secrets/global.secrets 1 2 3 4
python -m ectf25_design.gen_subscription secrets/global.secrets subscription1.bin 0xDEADBEEF 0 18446744073709551615 1
python -m ectf25_design.gen_subscription secrets/global.secrets subscription2.bin 0xDEADBEEF 0 18446744073709551615 2
python -m ectf25_design.gen_subscription secrets/global.secrets subscription3.bin 0xDEADBEEF 0 18446744073709551615 3
python -m ectf25_design.gen_subscription secrets/global.secrets subscription4.bin 0xDEADBEEF 0 18446744073709551615 4
