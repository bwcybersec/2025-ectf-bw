#!/bin/bash
pandoc -t markdown -o designdoc.md -F mermaid-filter 1-introduction.md 2-protocol.md 3-storage.md
pandoc -t pdf --pdf-engine tectonic -F mermaid-filter -o designdoc.pdf designdoc.md