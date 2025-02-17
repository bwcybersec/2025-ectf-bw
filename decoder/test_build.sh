#!/bin/bash
docker run --rm -v ./build_out:/out -v ./:/decoder -v ../secrets/global.secrets:/global.secrets:ro -e DECODER_ID=0xdeadbeef decoder
