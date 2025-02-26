#!/bin/bash
pandoc -t markdown -o designdoc.md 1-introduction.md 2-protocol.md 3-secrets.md 4-secreqs.md 5-storage.md 
pandoc -t pdf --pdf-engine tectonic -o ../design_BWU.pdf designdoc.md