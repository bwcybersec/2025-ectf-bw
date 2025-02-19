#!/bin/bash
mkdir -p secrets
rm secrets/secrets.json
rm secrets/global.secrets
rm subscription{1,2,3,4,6,7,8,9}.bin
python -m ectf25_design.gen_secrets secrets/global.secrets 1 2 3 4 6 7 8 9
python -m ectf25_design.gen_subscription secrets/global.secrets subscription1.bin 0xDEADBEEF 0 18446744073709551615 1
python -m ectf25_design.gen_subscription secrets/global.secrets subscription2.bin 0xDEADBEEF 0 18446744073709551615 2
python -m ectf25_design.gen_subscription secrets/global.secrets subscription3.bin 0xDEADBEEF 0 18446744073709551615 3
python -m ectf25_design.gen_subscription secrets/global.secrets subscription4.bin 0xDEADBEEF 0 18446744073709551615 4
python -m ectf25_design.gen_subscription secrets/global.secrets subscription6.bin 0xDEADBEEF 2345345 18446744079551615 6
python -m ectf25_design.gen_subscription secrets/global.secrets subscription7.bin 0xDEADBEEF 645645 18446744070951615 7
python -m ectf25_design.gen_subscription secrets/global.secrets subscription8.bin 0xDEADBEEF 23452345 184463709551615 8
python -m ectf25_design.gen_subscription secrets/global.secrets subscription9.bin 0xDEADBEEF 54345234 1873709551615 9
